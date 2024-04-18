use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
	punctuated, Error, FnArg, Pat, Result, Signature, Type, TypeReference,
};

use proc_macro_crate::{crate_name, FoundCrate};

pub(crate) fn validate_signature(sig: &Signature) -> Result<()> {
	if let Some(con) = &sig.constness {
		return Err(Error::new(con.span, "const not allowed"));
	}

	if let Some(unsf) = &sig.unsafety {
		return Err(Error::new(unsf.span, "unsafe not allowed"));
	}

	if let Some(abi) = &sig.abi {
		return Err(Error::new_spanned(abi, "abi not allowed"));
	}

	if !sig.generics.params.is_empty() {
		return Err(Error::new_spanned(&sig.generics, "generics not allowed"));
	}

	if let Some(variadic) = &sig.variadic {
		return Err(Error::new_spanned(&variadic, "variadic not allowed"));
	}

	Ok(())
}

#[allow(dead_code)]
pub(crate) fn ref_type(ty: &Type) -> Option<&TypeReference> {
	match ty {
		Type::Reference(r) => Some(r),
		_ => None,
	}
}

fn name_from_pattern(pat: &Pat) -> Option<String> {
	match pat {
		Pat::Ident(ident) => Some(ident.ident.to_string()),
		_ => None,
	}
}

#[allow(dead_code)]
pub(crate) fn validate_inputs(
	inputs: punctuated::Iter<'_, FnArg>,
) -> Result<Vec<(String, Box<Type>)>> {
	let mut v = vec![];

	for fn_arg in inputs {
		let (name, ty) = match fn_arg {
			FnArg::Receiver(r) => {
				return Err(Error::new_spanned(r, "self not allowed"))
			}
			FnArg::Typed(t) => {
				(name_from_pattern(&t.pat).unwrap_or_default(), t.ty.clone())
			}
		};

		if let Some(reff) = ref_type(&ty) {
			if let Some(lifetime) = &reff.lifetime {
				return Err(Error::new_spanned(
					lifetime,
					"lifetimes not \
					supported",
				));
			}
		}

		v.push((name, ty));
	}

	Ok(v)
}

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

#[cfg(feature = "api")]
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
