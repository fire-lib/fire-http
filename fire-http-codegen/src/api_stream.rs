use crate::route::generate_struct;
use crate::util::{
	fire_api_crate, ref_type, validate_inputs, validate_signature,
};
use crate::ApiArgs;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemFn, Result};

pub(crate) fn expand(args: ApiArgs, item: ItemFn) -> Result<TokenStream> {
	let fire_api = fire_api_crate()?;
	let fire = quote!(#fire_api::fire);
	let stream_mod = quote!(#fire_api::stream);
	let stream_ty = args.ty;

	validate_signature(&item.sig)?;

	// Box<Type>
	let inputs = validate_inputs(item.sig.inputs.iter())?;

	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	//
	let ty_as_stream = quote!(<#stream_ty as #stream_mod::Stream>);

	let into_stream_impl = quote!(
		impl #stream_mod::server::IntoStreamHandler for #struct_name {
			type Stream = #stream_ty;
			type Handler = #struct_name;

			fn into_handler(self) -> Self::Handler { self }
		}
	);

	let valid_data_fn = {
		let mut asserts = vec![];

		for (name, ty) in &inputs {
			let valid_fn = match ref_type(&ty) {
				Some(reff) => {
					let elem = &reff.elem;
					quote!(
						#stream_mod::util::valid_stream_data_as_ref
							::<#elem, #stream_ty>
					)
				}
				None => quote!(
					#stream_mod::util::valid_stream_data_as_owned
						::<#ty, #stream_ty>
				),
			};

			let error_msg = format!("could not find {}", quote!(#ty));

			asserts.push(quote!(
				assert!(#valid_fn(#name, params, data), #error_msg);
			));
		}

		quote!(
			fn validate_data(
				&self,
				params: &#fire::routes::ParamsNames,
				data: &#fire::Data
			) {
				#(#asserts)*
			}
		)
	};

	let handler_fn = {
		let asyncness = &item.sig.asyncness;
		let inputs = &item.sig.inputs;
		let output = &item.sig.output;
		let block = &item.block;

		quote!(
			#asyncness fn handler( #inputs ) #output
				#block
		)
	};

	let handle_fn = {
		let is_async = item.sig.asyncness.is_some();
		let await_kw = if is_async { quote!(.await) } else { quote!() };

		let mut handler_args_vars = vec![];
		let mut handler_args = vec![];

		for (idx, (name, ty)) in inputs.iter().enumerate() {
			let get_fn = match ref_type(&ty) {
				Some(reff) => {
					let elem = &reff.elem;
					quote!(
						#stream_mod::util::get_stream_data_as_ref
							::<#elem, #stream_ty>
					)
				}
				None => {
					quote!(
						#stream_mod::util::get_stream_data_as_owned
							::<#ty, #stream_ty>
					)
				}
			};

			let var_name = format_ident!("handler_arg_{idx}");

			handler_args_vars.push(quote!(
				let #var_name = #get_fn(#name, &streamer, &req, params, &data);
			));
			handler_args.push(quote!(#var_name));
		}

		quote!(
			fn handle<'a>(
				&'a self,
				req: #stream_mod::message::MessageData,
				params: &'a #fire::routes::PathParams,
				streamer: #stream_mod::streamer::RawStreamer,
				data: &'a #fire::Data
			) -> #stream_mod::server::PinnedFuture<'a, std::result::Result<
				#stream_mod::message::MessageData,
				#stream_mod::error::UnrecoverableError
			>> {
				#handler_fn

				type __Error = #ty_as_stream::Error;

				async fn _handle(
					streamer: #stream_mod::streamer::RawStreamer,
					req: #stream_ty,
					params: &#fire::routes::PathParams,
					data: &#fire::Data
				) -> std::result::Result<(), __Error> {
					// transform streamer
					let streamer = #stream_mod::util::transform_streamer
						::<#stream_ty>(streamer);

					let mut req = #fire_api::util::DataManager::new(req);
					let mut streamer = #fire_api::util::DataManager::new(
						streamer
					);

					#(#handler_args_vars)*

					handler(
						#(#handler_args),*
					)#await_kw
				}

				#stream_mod::server::PinnedFuture::new(async move {
					let req = #stream_mod::util::deserialize_req(req)?;

					let r = _handle(streamer, req, params, data).await;
					#stream_mod::util::error_to_data::<#stream_ty>(r)
				})
			}
		)
	};

	Ok(quote!(
		#struct_gen

		#into_stream_impl

		impl #stream_mod::server::StreamHandler for #struct_name {
			#valid_data_fn

			#handle_fn
		}
	))
}
