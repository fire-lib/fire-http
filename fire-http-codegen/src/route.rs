use crate::{Method, TransformOutput};
use crate::Args;

use proc_macro2::{TokenStream, Ident, Span};
use syn::{Result, Error, ItemFn, Signature, FnArg, Type};
use quote::{quote, format_ident};

use proc_macro_crate::{crate_name, FoundCrate};


pub(crate) fn expand(
	args: Args,
	item: ItemFn,
	method: Method,
	output: TransformOutput
) -> Result<TokenStream> {
	let fire = fire_http_crate()?;

	validate_signature(&item.sig)?;


	// parse inputs to get the data types
	// should ignore request and check that the request gets passed

	// (is_mut, ty)
	// TypeReference
	let mut input_types = vec![];

	for fn_arg in item.sig.inputs.iter() {
		let ty = match fn_arg {
			FnArg::Receiver(r) => {
				return Err(Error::new_spanned(r, "self not allowed"))
			},
			FnArg::Typed(t) => &t.ty
		};

		let reff = match &**ty {
			Type::Reference(r) => r,
			_ => return Err(Error::new_spanned(ty, "route argument needs to be \
				a reference"))
		};

		if let Some(lifetime) = &reff.lifetime {
			return Err(Error::new_spanned(lifetime, "lifetimes not allowed"))
		}

		input_types.push(reff);
	}

	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	let check_fn = {
		let (dyn_route, uri) = detect_dyn_uri(&args.uri);

		let uri_check = if dyn_route {
			quote!(req.uri().path().starts_with(#uri))
		} else {
			quote!(#fire::routes::check_static(req.uri().path(), #uri))
		};

		let method = format_ident!("{}", method.as_str());

		quote!(
			fn check(&self, req: &#fire::header::RequestHeader) -> bool {
				req.method() == &#fire::header::Method::#method &&
				#uri_check
			}
		)
	};

	let valid_data_fn = {
		let mut asserts = vec![];

		for ty in &input_types {
			let elem = &ty.elem;
			let valid_fn = if ty.mutability.is_some() {
				quote!(#fire::routes::util::valid_route_data_as_mut)
			} else {
				quote!(#fire::routes::util::valid_route_data_as_ref)
			};

			let error_msg = format!("could not find {}", quote!(#elem));

			asserts.push(quote!(
				assert!(#valid_fn::<#elem>(data), #error_msg);
			));
		}

		quote!(
			fn validate_data(&self, data: &#fire::Data) {
				#(#asserts)*
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

		for ty in &input_types {
			let elem = &ty.elem;
			let get_fn = if ty.mutability.is_some() {
				quote!(#fire::routes::util::get_route_data_as_mut)
			} else {
				quote!(#fire::routes::util::get_route_data_as_ref)
			};

			call_route_args.push(quote!(
				#get_fn::<#elem>(data, &mut req)
			));
		}

		let process_ret_ty = match output {
			TransformOutput::No => quote!(
				#fire::into::IntoRouteResult::into_route_result(ret)
			),
			TransformOutput::Json => quote!(
				let ret = #fire::json::IntoRouteResult::into_route_result(ret)?;
				#fire::json::serialize_to_response(&ret)
			)
		};

		quote!(
			fn call<'a>(
				&'a self,
				req: &'a mut #fire::Request,
				data: &'a #fire::Data
			) -> #fire::util::PinnedFuture<'a, #fire::Result<#fire::Response>> {
				#route_fn

				#fire::util::PinnedFuture::new(async move {
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
			#check_fn

			#valid_data_fn

			#call_fn
		}
	))
}

pub(crate) fn validate_signature(sig: &Signature) -> Result<()> {
	if let Some(con) = &sig.constness {
		return Err(Error::new(con.span, "const not allowed"))
	}

	if let Some(unsf) = &sig.unsafety {
		return Err(Error::new(unsf.span, "unsafe not allowed"))
	}

	if let Some(abi) = &sig.abi {
		return Err(Error::new_spanned(abi, "abi not allowed"))
	}

	if !sig.generics.params.is_empty() {
		return Err(Error::new_spanned(&sig.generics, "generics not allowed"))
	}

	if let Some(variadic) = &sig.variadic {
		return Err(Error::new_spanned(&variadic, "variadic not allowed"))
	}

	Ok(())
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

pub(crate) fn detect_dyn_uri(args_uri: &str) -> (bool, String) {
	let uri = args_uri.strip_suffix('*');
	let dyn_route = uri.is_some();
	let uri = uri.unwrap_or_else(|| args_uri)
		.to_string();

	(dyn_route, uri)
}

// Util Functions

pub(crate) fn fire_http_crate() -> Result<TokenStream> {
	let name = crate_name("fire-http")
		.map_err(|e| Error::new(Span::call_site(), e))?;

	Ok(match name {
		// if it get's used inside fire_http it is a test or an example
		FoundCrate::Itself => quote!(fire_http),
		FoundCrate::Name(n) => {
			let ident = Ident::new(&n, Span::call_site());
			quote!(#ident)
		}
	})
}

#[allow(dead_code)]
fn call_site_err(msg: &'static str) -> Error {
	Error::new(Span::call_site(), msg)
}