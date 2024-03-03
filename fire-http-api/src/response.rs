use fire::header::HeaderValues;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub struct ResponseHeaders(pub(crate) HeaderValues);

impl ResponseHeaders {
	#[doc(hidden)]
	pub fn new() -> Self {
		Self(HeaderValues::new())
	}
}

impl Deref for ResponseHeaders {
	type Target = HeaderValues;

	fn deref(&self) -> &HeaderValues {
		&self.0
	}
}

impl DerefMut for ResponseHeaders {
	fn deref_mut(&mut self) -> &mut HeaderValues {
		&mut self.0
	}
}
