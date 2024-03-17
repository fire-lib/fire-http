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
	let req_ty = args.ty;

	validate_signature(&item.sig)?;

	// Box<Type>
	let inputs = validate_inputs(item.sig.inputs.iter(), true)?;

	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	//
	let ty_as_req = quote!(<#req_ty as #fire_api::Request>);

	let valid_data_fn = {
		let mut asserts = vec![];

		for (name, ty) in &inputs {
			let error_msg = format!("could not find {}", quote!(#ty));

			let valid_fn = match ref_type(&ty) {
				Some(reff) if reff.mutability.is_some() => {
					let elem = &reff.elem;
					quote!(
						#fire_api::util::valid_route_data_as_mut
							::<#elem, #req_ty>
					)
				}
				Some(reff) => {
					let elem = &reff.elem;
					quote!(
						#fire_api::util::valid_route_data_as_ref
							::<#elem, #req_ty>
					)
				}
				None => quote!(
					#fire_api::util::valid_route_data_as_owned
						::<#ty, #req_ty>
				),
			};

			asserts.push(quote!(
				assert!(#valid_fn(#name, params, data), #error_msg);
			));
		}

		quote!(
			fn validate_data(&self, params: &#fire::routes::ParamsNames, data: &#fire::Data) {
				#(#asserts)*
			}
		)
	};

	// path fn
	let path_fn = quote!(
		fn path(&self) -> #fire::routes::RoutePath {
			#fire::routes::RoutePath {
				method: Some(#ty_as_req::METHOD),
				path: #ty_as_req::PATH.into()
			}
		}
	);

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

	let call_fn = {
		let is_async = item.sig.asyncness.is_some();
		let await_kw = if is_async { quote!(.await) } else { quote!() };

		let mut handler_args_vars = vec![];
		let mut handler_args = vec![];

		for (idx, (name, ty)) in inputs.iter().enumerate() {
			let get_fn = match ref_type(&ty) {
				Some(reff) if reff.mutability.is_some() => {
					let elem = &reff.elem;
					quote!(
						#fire_api::util::get_route_data_as_mut
							::<#elem, #req_ty>
					)
				}
				Some(reff) => {
					let elem = &reff.elem;
					quote!(
						#fire_api::util::get_route_data_as_ref
							::<#elem, #req_ty>
					)
				}
				None => quote!(
					#fire_api::util::get_route_data_as_owned
						::<#ty, #req_ty>
				),
			};

			let var_name = format_ident!("handler_arg_{idx}");

			handler_args_vars.push(quote!(
				let #var_name = #get_fn(#name, &req, &header, params, &resp_header, data);
			));
			handler_args.push(quote!(#var_name));
		}

		quote!(
			fn call<'a>(
				&'a self,
				req: &'a mut #fire::Request,
				params: &'a #fire::routes::PathParams,
				data: &'a #fire::Data
			) -> #fire::util::PinnedFuture<'a, #fire::Result<#fire::Response>> {
				#handler_fn

				type __Response = #ty_as_req::Response;
				type __Error = #ty_as_req::Error;

				async fn route_to_body(
					fire_req: &mut #fire::Request,
					params: &#fire::routes::PathParams,
					data: &#fire::Data
				) -> std::result::Result<(
					#fire_api::response::ResponseSettings,
					#fire::Body
				), __Error> {
					#fire_api::util::setup_request::<#req_ty>(fire_req)?;

					let req = #fire_api::util::deserialize_req::<#req_ty>(
						fire_req
					).await?;

					let req = #fire_api::util::DataManager::new(req);
					let header = fire_req.header();
					let resp_header = #fire_api::util::DataManager::new(
						#fire_api::response::ResponseSettings::new()
					);

					#(#handler_args_vars)*

					let resp: __Response = handler(
							#(#handler_args),*
					)#await_kw?;

					#fire_api::util::serialize_resp::<#req_ty>(&resp)
						.map(|body| (resp_header.take_owned(), body))
				}

				#fire::util::PinnedFuture::new(async move {
					#fire_api::util::transform_body_to_response::<#req_ty>(
						route_to_body(req, params, data).await
					)
				})
			}
		)
	};

	Ok(quote!(
		#struct_gen

		impl #fire::routes::Route for #struct_name {
			#valid_data_fn

			#path_fn

			#call_fn
		}
	))
}
