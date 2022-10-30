use crate::ApiArgs;
use crate::route::{validate_signature, generate_struct};

use proc_macro2::{TokenStream, Span, Ident};
use syn::{Result, Error, ItemFn, FnArg, Type, TypeReference};
use quote::quote;

use proc_macro_crate::{crate_name, FoundCrate};


fn ref_type(ty: &Type) -> Option<&TypeReference> {
	match ty {
		Type::Reference(r) => Some(r),
		_ => None
	}
}


pub(crate) fn expand(
	args: ApiArgs,
	item: ItemFn
) -> Result<TokenStream> {
	let fire_api = fire_api_crate()?;
	let fire = quote!(#fire_api::fire);
	let req_ty = args.ty;

	validate_signature(&item.sig)?;

	// Box<Type>
	let mut input_types = vec![];

	for fn_arg in item.sig.inputs.iter() {
		let ty = match fn_arg {
			FnArg::Receiver(r) => {
				return Err(Error::new_spanned(r, "self not allowed"))
			},
			FnArg::Typed(t) => &t.ty
		};

		if let Some(reff) = ref_type(&ty) {
			if let Some(mu) = reff.mutability {
				return Err(Error::new_spanned(mu, "&mut not supported"))
			}

			if let Some(lifetime) = &reff.lifetime {
				return Err(Error::new_spanned(lifetime, "lifetimes not \
					supported"))
			}
		}

		input_types.push(ty);
	}


	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	//
	let ty_as_req = quote!(<#req_ty as #fire_api::Request>);

	// check fn
	let check_fn = quote!(
		fn check(&self, req: &#fire::header::RequestHeader) -> bool {
			let method = #ty_as_req::METHOD;
			let uri = #ty_as_req::PATH;

			req.method() == &method &&
			#fire::routes::check_static(req.uri().path(), uri)
		}
	);

	let valid_data_fn = {
		let mut asserts = vec![];

		for ty in &input_types {
			let error_msg = format!("could not find {}", quote!(#ty));

			let valid_fn = match ref_type(&ty) {
				Some(reff) => {
					let elem = &reff.elem;
					quote!(
						#fire_api::util::valid_route_data_as_ref
							::<#elem, #req_ty>
					)
				},
				None => quote!(
					#fire_api::util::valid_route_data_as_owned
						::<#ty, #req_ty>
				)
			};

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

		let mut handler_args = vec![];

		for ty in &input_types {
			let get_fn = match ref_type(&ty) {
				Some(reff) => {
					let elem = &reff.elem;
					quote!(
						#fire_api::util::get_route_data_as_ref
							::<#elem, #req_ty>
					)
				},
				None => quote!(
					#fire_api::util::get_route_data_as_owned
						::<#ty, #req_ty>
				)
			};

			handler_args.push(quote!(
				#get_fn(data, &header, &mut req)
			));
		}

		quote!(
			fn call<'a>(
				&'a self,
				req: &'a mut #fire::Request,
				data: &'a #fire::Data
			) -> #fire::util::PinnedFuture<'a, #fire::Result<#fire::Response>> {
				#handler_fn

				type __Response = #ty_as_req::Response;
				type __Error = #ty_as_req::Error;

				async fn route_to_body(
					fire_req: &mut #fire::Request,
					data: &#fire::Data
				) -> std::result::Result<#fire::Body, __Error> {
					#fire_api::util::setup_request::<#req_ty>(fire_req)?;

					let req = #fire_api::util::deserialize_req::<#req_ty>(
						fire_req
					).await?;

					let mut req = Some(req);
					let header = fire_req.header();

					let resp: __Response = handler(
							#(#handler_args),*
					)#await_kw?;

					#fire_api::util::serialize_resp::<#req_ty>(&resp)
				}

				#fire::util::PinnedFuture::new(async move {
					#fire_api::util::transform_body_to_response::<#req_ty>(
						route_to_body(req, data).await
					)
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

pub(crate) fn fire_api_crate() -> Result<TokenStream> {
	let name = crate_name("fire-http-api")
		.map_err(|e| Error::new(Span::call_site(), e))?;

	Ok(match name {
		// if it get's used inside fire_http it is a test or an example
		FoundCrate::Itself => quote!(fire_http_api),
		FoundCrate::Name(n) => {
			let ident = Ident::new(&n, Span::call_site());
			quote!(#ident)
		}
	})
}