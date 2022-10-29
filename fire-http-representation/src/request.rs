use crate::header::RequestHeader;
use crate::body::Body;

use std::time::Duration;


/// The request that is received from a client.
#[derive(Debug)]
pub struct Request {
	pub header: RequestHeader,
	pub body: Body
}

impl Request {
	/// Creates a new `Request`.
	pub fn new(header: RequestHeader, body: Body) -> Self {
		Self { header, body }
	}

	/// Takes the body replacing it with an empty one.
	pub fn take_body(&mut self) -> Body {
		self.body.take()
	}

	/// Get the request header by reference.
	pub fn header(&self) -> &RequestHeader {
		&self.header
	}

	/// Sets a read size limit.
	pub fn set_size_limit(&mut self, size: Option<usize>) {
		self.body.set_size_limit(size)
	}

	/// Sets a read timeout, the timer starts counting after you call into_*
	pub fn set_timeout(&mut self, timeout: Option<Duration>) {
		self.body.set_timeout(timeout)
	}

	/// Tries to deserialize the request body.
	/// 
	/// ## Errors
	/// - If the header `content-type` does not contain `application/json`.
	/// - If the body does not contain a valid json or some data is missing.
	#[cfg(feature = "json")]
	pub async fn deserialize<D>(&mut self) -> Result<D, DeserializeError>
	where D: serde::de::DeserializeOwned + Send + 'static {
		use crate::header::Mime;

		// try to read mime
		// this will not work if content-type has charset
		// TODO allow charset (probably implement Parse for ContentType)
		let raw_content_type = self.header()
			.value("content-type")
			.ok_or(DeserializeError::NoContentType)?;
		let mime: Mime = raw_content_type.trim().parse()
			.map_err(|_| DeserializeError::UnknownContentType(
				raw_content_type.to_string()
			))?;

		if mime != Mime::JSON {
			return Err(DeserializeError::WrongMimeType(mime))
		}

		// now parse body
		self.body.take().deserialize().await
			.map_err(|e| DeserializeError::Reading(e))
	}
}

#[cfg(feature = "json")]
mod deserialize_error {
	use crate::header::Mime;

	use std::{io, fmt};


	#[derive(Debug)]
	#[non_exhaustive]
	pub enum DeserializeError {
		NoContentType,
		UnknownContentType(String),
		WrongMimeType(Mime),
		Reading(io::Error)
	}

	impl fmt::Display for DeserializeError {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			write!(f, "Failed to deserialize requets with error {:?}", self)
		}
	}

	impl std::error::Error for DeserializeError {}
}

#[cfg(feature = "json")]
pub use deserialize_error::*;