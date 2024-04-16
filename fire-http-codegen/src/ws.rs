use crate::route::generate_struct;
use crate::util::{
	fire_http_crate, ref_type, validate_inputs, validate_signature,
};
use crate::Args;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemFn, Result};

pub(crate) fn expand(args: Args, item: ItemFn) -> Result<TokenStream> {
	let fire = fire_http_crate()?;

	validate_signature(&item.sig)?;

	// Box<Type>
	let inputs = validate_inputs(item.sig.inputs.iter(), false)?;

	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	let valid_data_fn = {
		let mut asserts = vec![];

		for (name, ty) in &inputs {
			let valid_fn = match ref_type(&ty) {
				Some(reff) => {
					let elem = &reff.elem;
					quote!(#fire::ws::util::valid_ws_data_as_ref::<#elem>)
				}
				None => {
					quote!(#fire::ws::util::valid_ws_data_as_owned::<#ty>)
				}
			};

			let error_msg = format!("could not find {}", quote!(#ty));

			asserts.push(quote!(
				assert!(#valid_fn(#name, params, data), #error_msg);
			));
		}

		quote!(
			fn validate_data(&self, params: &#fire::routes::ParamsNames, data: &#fire::Resources) {
				#(#asserts)*
			}
		)
	};

	let path_fn = {
		let uri = &args.uri;

		quote!(
			fn path(&self) -> #fire::routes::RoutePath {
				#fire::routes::RoutePath {
					method: Some(#fire::header::Method::GET),
					path: #uri.into()
				}
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

	let call_fn = {
		let is_async = item.sig.asyncness.is_some();
		let await_kw = if is_async { quote!(.await) } else { quote!() };

		let mut handler_args_vars = vec![];
		let mut handler_args = vec![];

		for (idx, (name, ty)) in inputs.iter().enumerate() {
			let get_fn = match ref_type(&ty) {
				Some(reff) => {
					let elem = &reff.elem;
					quote!(#fire::ws::util::get_ws_data_as_ref::<#elem>)
				}
				None => {
					quote!(#fire::ws::util::get_ws_data_as_owned::<#ty>)
				}
			};

			let var_name = format_ident!("handler_arg_{idx}");

			handler_args_vars.push(quote!(
				let #var_name = #get_fn(#name, &ws, &params, &data);
			));
			handler_args.push(quote!(#var_name));
		}

		quote!(
			fn call<'a>(
				&'a self,
				req: &'a mut #fire::routes::HyperRequest,
				params: &'a #fire::routes::PathParams,
				data: &'a #fire::Resources
			) -> #fire::util::PinnedFuture<'a,
				Option<#fire::Result<#fire::Response>>
			> {
				#handler_fn

				#fire::util::PinnedFuture::new(async move {
					let upgrade = #fire::ws::util::upgrade(req);
					let (on_upgrade, ws_accept) = match upgrade {
						Ok(o) => o,
						Err(e) => return Some(Err(e))
					};

					let data = data.clone();
					let params = params.clone();

					#fire::ws::util::spawn(async move {
						match on_upgrade.await {
							Ok(upgraded) => {
								let ws = #fire::ws::WebSocket::new(
									upgraded
								).await;
								let ws = #fire::ws::util::DataManager::new(ws);

								#(#handler_args_vars)*

								let ret = handler(
									#(#handler_args),*
								)#await_kw;

								#fire::ws::util::log_websocket_return(ret);
							},
							Err(e) => #fire::ws::util::upgrade_error(e)
						}
					});

					Some(Ok(#fire::ws::util::switching_protocols(ws_accept)))
				})
			}
		)
	};

	Ok(quote!(
		#struct_gen

		impl #fire::routes::RawRoute for #struct_name {
			#valid_data_fn

			#path_fn

			#call_fn
		}
	))
}
