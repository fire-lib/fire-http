use fire::{
	header::{values::IntoHeaderName, HeaderValue, HeaderValues, StatusCode},
	state::StateRefCell,
};

use std::fmt;

#[derive(Debug, Clone)]
pub struct ResponseSettings {
	pub(crate) headers: HeaderValues,
	pub(crate) status: StatusCode,
}

impl ResponseSettings {
	#[doc(hidden)]
	pub fn new() -> Self {
		Self {
			headers: HeaderValues::new(),
			status: StatusCode::OK,
		}
	}

	#[doc(hidden)]
	pub fn new_for_state() -> StateRefCell<Self> {
		StateRefCell::new(Self::new())
	}

	pub fn headers_mut(&mut self) -> &mut HeaderValues {
		&mut self.headers
	}

	/// Sets a header value.
	///
	/// ## Note
	/// Only ASCII characters are allowed, use
	/// `self.headers_mut().encode_value()` to allow any character.
	///
	/// ## Panics
	/// If the value is not a valid `HeaderValue`.
	pub fn header<K, V>(&mut self, key: K, val: V) -> &mut Self
	where
		K: IntoHeaderName,
		V: TryInto<HeaderValue>,
		V::Error: fmt::Debug,
	{
		self.headers.insert(key, val);
		self
	}

	pub fn status(&mut self, status: StatusCode) -> &mut Self {
		self.status = status;
		self
	}
}
