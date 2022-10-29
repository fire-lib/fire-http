mod builder;
pub use builder::ResponseBuilder;

use crate::header::{ResponseHeader, StatusCode};
use crate::body::Body;

/// The response created from a server.
#[derive(Debug)]
pub struct Response {
	pub header: ResponseHeader,
	// if you overide the body
	// you should pobably reset the content-length
	pub body: Body
}

impl Response {

	/// Creates a new `Response`.
	pub fn new(header: ResponseHeader, body: Body) -> Self {
		Self { header, body }
	}

	/// Creates a new `Response` with a builder.
	pub fn builder() -> ResponseBuilder {
		ResponseBuilder::new()
	}

	/// Get the response header by reference.
	pub fn header(&self) -> &ResponseHeader {
		&self.header
	}

	/// Takes the body replacing it with an empty one.
	/// 
	/// ## Note
	/// If you used the builder to create a `Response`
	/// you should probably reset the `content-length` header.
	pub fn take_body(&mut self) -> Body {
		self.body.take()
	}
}

impl From<Body> for Response {
	fn from(body: Body) -> Self {
		Self::builder()
			.body(body)
			.build()
	}
}

impl From<StatusCode> for Response {
	fn from(status_code: StatusCode) -> Self {
		Self::builder()
			.status_code(status_code)
			.build()
	}
}