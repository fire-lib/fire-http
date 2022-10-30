use syn::parse::{Parse, ParseStream, Result};
use syn::LitStr;


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