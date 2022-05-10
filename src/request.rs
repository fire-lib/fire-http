
use crate::util::convert_hyper_req_to_fire_req;
use crate::fire::RequestConfigs;
use crate::util::HeaderError;

use std::net::SocketAddr;
use std::mem;
use std::time::Duration;

use http::header::RequestHeader;
use http::body::{Body, BodyWithTimeout};

pub type HyperRequest = hyper::Request<hyper::Body>;

/// The Request that get's returned to every route.
#[derive(Debug)]
pub struct Request {
	header: RequestHeader,
	body: BodyWithTimeout
}

impl Request {

	pub(crate) fn new(header: RequestHeader, body: BodyWithTimeout) -> Self {
		Self {header, body}
	}

	/// Takes the current body and leaves the request with an empty body.
	pub fn take_body(&mut self) -> BodyWithTimeout {
		self.body.take()
	}

	/// Takes the current body and leaves the request an empty body.
	/// 
	/// This discard the timout associated with the body.
	pub fn take_raw_body(&mut self) -> Body {
		self.body.body_mut().take()
	}

	/// Checks if the request body is empty.
	pub fn is_empty(&self) -> bool {
		self.body.is_empty()
	}

	/// Returns a reference to the body contained in the request.
	pub fn body(&self) -> &BodyWithTimeout {
		&self.body
	}

	/// Returns a mutable reference to the body contained in the request.
	pub fn body_mut(&mut self) -> &mut BodyWithTimeout {
		&mut self.body
	}

	/// Returns a reference to the body contained in the request without a
	/// timeout being associated with it.
	pub fn raw_body(&self) -> &Body {
		self.body.body()
	}

	/// Returns a reference to the body contained in the request without a
	/// timeout being associated with it.
	pub fn raw_body_mut(&mut self) -> &mut Body {
		self.body.body_mut()
	}

	/// Changes the size limit of the current request.
	pub fn set_size_limit(&mut self, size_limit: usize) {
		self.body.set_size_limit(size_limit);
	}

	/// Changes the timeout of the current request.
	pub fn set_timeout(&mut self, duration: Duration) {
		self.body.set_timeout(duration);
	}

	/// Returns a reference to the request header.
	pub fn header(&self) -> &RequestHeader {
		&self.header
	}

	/// Returns a mutable reference to the request header.
	pub fn header_mut(&mut self) -> &mut RequestHeader {
		&mut self.header
	}

	/// Tries to deserialize the request body.
	/// 
	/// ## Errors
	/// - If the header `content-type` does not contain `application/json`.
	/// - If the body does not contain a valid json or some data is missing.
	///
	/// ## Note
	/// The request will now contain an empty body.
	//
	// this is a client error
	// because well, its in request
	#[cfg(feature = "json")]
	pub async fn deserialize<D>(&mut self) -> crate::Result<D>
	where D: serde::de::DeserializeOwned {
		use crate::error::{Error, ClientErrorKind};
		use http::header::Mime;
		use http::body::JsonError;
		// try to read mime
		// this will not work if content-type has charset
		// TODO allow charset (probably implement Parse for ContentType)
		let raw_content_type = self.header()
			.value("content-type")
			.ok_or(DeserializeError::NoContentType)?;
		let mime = Mime::try_from_mime(raw_content_type)
			.ok_or_else(|| DeserializeError::UnknownContentType(
				raw_content_type.to_string()
			))?;

		if !matches!(mime, Mime::Json) {
			return Err(DeserializeError::WrongMimeType(mime).into())
		}

		// now parse body
		self.body.take().deserialize().await
			.map_err(|e| match e {
				JsonError::IoError(e) => {
					Error::new(ClientErrorKind::BadRequest, e)
				},
				JsonError::SerdeJson(e) => DeserializeError::JsonError(e).into()
			})
	}

}

#[cfg(feature = "json")]
mod json_error {

	use crate::error::{Error, ClientErrorKind};
	use std::fmt;
	use http::header::Mime;

	#[derive(Debug)]
	pub enum DeserializeError {
		NoContentType,
		UnknownContentType(String),
		WrongMimeType(Mime),
		JsonError(serde_json::Error)
	}

	impl From<DeserializeError> for Error {
		fn from(e: DeserializeError) -> Self {
			Self::new(ClientErrorKind::BadRequest, e)
		}
	}

	impl fmt::Display for DeserializeError {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			fmt::Debug::fmt(self, f)
		}
	}

	impl std::error::Error for DeserializeError {
		fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
			match self {
				Self::JsonError(e) => Some(e),
				_ => None
			}
		}
	}
}

#[cfg(feature = "json")]
use json_error::DeserializeError;


/// a container that contains a HyperRequest
/// or a Fire Request
/// if you receive this in a RawRoute
/// you always receiver a hyper request
#[derive(Debug)]
pub enum RequestBuilder<'a> {
	Hyper {
		hyper_req: HyperRequest,
		address: SocketAddr,
		configs: &'a RequestConfigs
	},
	Fire(Request),
	ConvertError(HeaderError)
}

impl<'a> RequestBuilder<'a> {

	pub(crate) fn new(
		hyper_req: HyperRequest,
		address: SocketAddr,
		configs: &'a RequestConfigs
	) -> Self {
		Self::Hyper {
			hyper_req,
			address,
			configs
		}
	}

	/// if you receive this in a raw request
	/// you can unwrap
	#[inline]
	pub fn hyper_mut(&mut self) -> Option<&mut HyperRequest> {
		match self {
			Self::Hyper { hyper_req, .. } => Some(hyper_req),
			_ => None
		}
	}

	/// if you receive this in a raw request
	/// you can unwrap
	#[inline]
	pub fn hyper_ref(&self) -> Option<&HyperRequest> {
		match self {
			Self::Hyper { hyper_req, .. } => Some(hyper_req),
			_ => None
		}
	}

	/// if this is called once
	/// the hyper request is gone
	/// if this returns Error
	/// You are not allowed to call it again
	/// it will panic
	pub fn fire_mut(&mut self) -> Result<&mut Request, HeaderError> {
		if let Self::Fire(f) = self {
			return Ok(f)
		}

		let this = mem::replace(self, Self::ConvertError(HeaderError::Unknown));
		match this.into_fire() {
			Ok(r) => {
				let _ = mem::replace(self, Self::Fire(r));
				Ok(match self {
					Self::Fire(r) => r,
					_ => unreachable!()
				})
			},
			Err(e) => {
				let _ = mem::replace(self, Self::ConvertError(e.clone()));
				Err(e)
			}
		}
	}

	pub fn into_fire(self) -> Result<Request, HeaderError> {
		match self {
			Self::Fire(r) => Ok(r),
			Self::Hyper { hyper_req, address, configs } => {
				convert_hyper_req_to_fire_req(hyper_req, address, configs)
			},
			Self::ConvertError(e) => Err(e)
		}
	}

}


