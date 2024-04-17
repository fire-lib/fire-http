use crate::util::{fire_http_crate, validate_signature};
use crate::Args;
use crate::{Method, TransformOutput};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, FnArg, ItemFn, Pat, Result, Type};

fn name_from_pattern(pat: &Pat) -> Option<String> {
	match pat {
		Pat::Ident(ident) => Some(ident.ident.to_string()),
		_ => None,
	}
}

pub(crate) fn expand(
	args: Args,
	item: ItemFn,
	method: Method,
	output: TransformOutput,
) -> Result<TokenStream> {
	let fire = fire_http_crate()?;

	validate_signature(&item.sig)?;

	// parse inputs to get the data types
	// should ignore request and check that the request gets passed

	// (is_mut, ty)
	// TypeReference
	let mut inputs = vec![];

	for fn_arg in item.sig.inputs.iter() {
		let (name, ty) = match fn_arg {
			FnArg::Receiver(r) => {
				return Err(Error::new_spanned(r, "self not allowed"))
			}
			FnArg::Typed(t) => {
				(name_from_pattern(&t.pat).unwrap_or_else(String::new), &t.ty)
			}
		};

		if let Type::Reference(reff) = &**ty {
			if let Some(lifetime) = &reff.lifetime {
				return Err(Error::new_spanned(
					lifetime,
					"lifetimes not allowed",
				));
			}
		}

		inputs.push((name, ty));
	}

	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	let valid_data_fn = {
		let mut asserts = vec![];

		for (name, ty) in &inputs {
			// let valid_fn = if ty.mutability.is_some() {
			// 	quote!(#fire::routes::util::valid_route_data_as_mut)
			// } else {
			// 	quote!(#fire::routes::util::valid_route_data_as_ref)
			// };

			let error_msg = format!("could not find {}: {}", name, quote!(#ty));

			asserts.push(quote!(
				<#ty as #fire::extractor::Extractor>::validate_requirements(
					#name, params, resources
				);
				// assert!(#valid_fn::<#elem>(#name, params, data), #error_msg);
			));
		}

		quote!(
			fn validate_requirements(
				&self,
				params: &#fire::routes::ParamsNames,
				resources: &#fire::resources::Resources
			) {
				#(#asserts)*
			}
		)
	};

	let path_fn = {
		let uri = &args.uri;
		let method = format_ident!("{}", method.as_str());

		quote!(
			fn path(&self) -> #fire::routes::RoutePath {
				#fire::routes::RoutePath {
					method: Some(#fire::header::Method::#method),
					path: #uri.into()
				}
			}
		)
	};

	let route_fn = {
		let asyncness = &item.sig.asyncness;
		let inputs = &item.sig.inputs;
		let output = &item.sig.output;
		let block = &item.block;

		quote!(
			#asyncness fn handle_route( #inputs ) #output
				#block
		)
	};

	let call_fn = {
		let async_route_fn = item.sig.asyncness.is_some();
		let await_kw = if async_route_fn {
			quote!(.await)
		} else {
			quote!()
		};

		let mut call_route_args = vec![];
		let mut prepare_extractors = vec![];

		for (name, ty) in &inputs {
			prepare_extractors.push(quote!(
				let res = <#ty as #fire::extractor::Extractor>::prepare(
					#name,
					req,
					params,
					&mut state,
					resources
				).await;

				if let Err(e) = res {
					return Err(#fire::Error::new(
						#fire::extractor::ExtractorError::error_kind(&e),
						#fire::extractor::ExtractorError::into_std(e)
					));
				}
			));

			call_route_args.push(quote!({
				let res = <#ty as #fire::extractor::Extractor>::extract(
					#name,
					&mut req,
					params,
					&state,
					resources
				);

				match res {
					Ok(res) => res,
					Err(err) => return Err(#fire::Error::new(
						#fire::extractor::ExtractorError::error_kind(&err),
						#fire::extractor::ExtractorError::into_std(err)
					))
				}
			}));
		}

		let process_ret_ty = match output {
			TransformOutput::No => quote!(
				#fire::into::IntoRouteResult::into_route_result(ret)
			),
			TransformOutput::Json => quote!(
				let ret = #fire::json::IntoRouteResult::into_route_result(ret)?;
				#fire::json::serialize_to_response(&ret)
			),
		};

		quote!(
			fn call<'a>(
				&'a self,
				req: &'a mut #fire::Request,
				params: &'a #fire::routes::PathParams,
				resources: &'a #fire::resources::Resources
			) -> #fire::util::PinnedFuture<'a, #fire::Result<#fire::Response>> {
				#route_fn

				#fire::util::PinnedFuture::new(async move {
					#[allow(unused_mut, dead_code)]
					let mut state = #fire::state::State::new();

					// prepare extractions
					#(#prepare_extractors)*

					let mut req = Some(req);

					let ret = handle_route(
						#(#call_route_args),*
					)#await_kw;

					#process_ret_ty
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

pub(crate) fn generate_struct(item: &ItemFn) -> TokenStream {
	let struct_name = &item.sig.ident;
	let attrs = &item.attrs;
	let vis = &item.vis;

	quote!(
		#(#attrs)*
		#[allow(non_camel_case_types)]
		#vis struct #struct_name;
	)
}
