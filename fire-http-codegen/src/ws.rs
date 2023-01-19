use crate::Args;
use crate::route::{generate_struct, detect_dyn_uri};
use crate::util::{
	fire_http_crate, validate_signature, validate_inputs, ref_type
};

use proc_macro2::TokenStream;
use syn::{Result, ItemFn};
use quote::{quote, format_ident};


pub(crate) fn expand(
	args: Args,
	item: ItemFn
) -> Result<TokenStream> {
	let fire = fire_http_crate()?;

	validate_signature(&item.sig)?;

	// Box<Type>
	let input_types = validate_inputs(item.sig.inputs.iter(), false)?;


	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	let check_fn = {
		let (dyn_route, uri) = detect_dyn_uri(&args.uri);

		let uri_check = if dyn_route {
			quote!(req.uri().path().starts_with(#uri))
		} else {
			quote!(#fire::routes::check_static(req.uri().path(), #uri))
		};

		quote!(
			fn check(&self, req: &#fire::routes::HyperRequest) -> bool {
				req.method() == &#fire::header::Method::GET &&
				#uri_check
			}
		)
	};

	let valid_data_fn = {
		let mut asserts = vec![];

		for ty in &input_types {
			let valid_fn = match ref_type(&ty) {
				Some(reff) => {
					let elem = &reff.elem;
					quote!(#fire::ws::util::valid_ws_data_as_ref::<#elem>)
				},
				None => {
					quote!(#fire::ws::util::valid_ws_data_as_owned::<#ty>)
				}
			};

			let error_msg = format!("could not find {}", quote!(#ty));

			asserts.push(quote!(
				assert!(#valid_fn(data), #error_msg);
			));
		}

		quote!(
			fn validate_data(&self, data: &#fire::Data) {
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

	let call_fn = {
		let is_async = item.sig.asyncness.is_some();
		let await_kw = if is_async {
			quote!(.await)
		} else {
			quote!()
		};

		let mut handler_args_vars = vec![];
		let mut handler_args = vec![];

		for (idx, ty) in input_types.iter().enumerate() {
			let get_fn = match ref_type(&ty) {
				Some(reff) => {
					let elem = &reff.elem;
					quote!(#fire::ws::util::get_ws_data_as_ref::<#elem>)
				},
				None => {
					quote!(#fire::ws::util::get_ws_data_as_owned::<#ty>)
				}
			};

			let var_name = format_ident!("handler_arg_{idx}");

			handler_args_vars.push(quote!(
				let #var_name = #get_fn(&data, &ws);
			));
			handler_args.push(quote!(#var_name));
		}

		quote!(
			fn call<'a>(
				&'a self,
				req: &'a mut #fire::routes::HyperRequest,
				data: &'a #fire::Data
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
			#check_fn

			#valid_data_fn

			#call_fn
		}
	))
}