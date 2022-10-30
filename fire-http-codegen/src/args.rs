use syn::parse::{Parse, ParseStream, Result};
use syn::LitStr;

#[cfg(feature = "api")]
pub(crate) use api::*;


#[derive(Debug, Clone)]
pub(crate) struct Args {
	pub uri: String
}

impl Parse for Args {
	fn parse(input: ParseStream) -> Result<Self> {
		// parse a string
		let uri: LitStr = input.parse()?;

		Ok(Self { uri: uri.value() })
	}
}

#[cfg(feature = "api")]
mod api {
	use super::*;

	use syn::Type;


	#[derive(Debug, Clone)]
	pub(crate) struct ApiArgs {
		pub ty: Type
	}

	impl Parse for ApiArgs {
		fn parse(input: ParseStream) -> Result<Self> {
			// parse a string
			let ty: Type = input.parse()?;

			Ok(Self { ty })
		}
	}
}