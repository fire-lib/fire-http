use std::net::SocketAddr;

pub use http::{StatusCode, Method, Uri};

pub mod url;
pub use url::Url;

mod contenttype;
pub use contenttype::{ContentType, Mime};

pub mod values;
pub use values::{HeaderValues, HeaderValue};


/// RequestHeader received from a client.
#[derive(Debug, Clone)]
pub struct RequestHeader {
	pub address: SocketAddr,
	pub method: Method,
	pub uri: Uri,
	pub values: HeaderValues
}

impl RequestHeader {
	/// Returns the ip address of the requesting client.
	pub fn address(&self) -> &SocketAddr {
		&self.address
	}

	/// Returns the requesting method.
	pub fn method(&self) -> &Method {
		&self.method
	}

	/// Returns the requesting uri.
	pub fn uri(&self) -> &Uri {
		&self.uri
	}

	pub fn to_url(&self) -> Option<Url> {
		Url::from_inner(self.uri.clone())
	}

	/// Returns all header values.
	pub fn values(&self) -> &HeaderValues {
		&self.values
	}

	/// Returns a header value from it's key if it exists and is valid ascii.
	/// 
	/// ## Note
	/// If you wan't a decoded value use `self.values().get_decoded(key)`.
	pub fn value<K>(&self, key: K) -> Option<&str>
	where K: values::AsHeaderName {
		self.values.get_str(key)
	}
}

/// ResponseHeader created from a server.
/// 
/// To create a ResponseHeader you should probably
/// use ResponseHeaderBuilder.
#[derive(Debug, Clone)]
pub struct ResponseHeader {
	pub status_code: StatusCode,
	pub content_type: ContentType,
	pub values: HeaderValues
}

impl ResponseHeader {
	/// Returns the used status code.
	pub fn status_code(&self) -> &StatusCode {
		&self.status_code
	}

	/// Returns the used content type.
	pub fn content_type(&self) -> &ContentType {
		&self.content_type
	}

	/// Returns all header values.
	pub fn values(&self) -> &HeaderValues {
		&self.values
	}

	/// Returns a header value from it's key if it exists and is valid ascii.
	/// 
	/// ## Note
	/// If you wan't a decoded value use `self.values().get_decoded(key)`.
	pub fn value<K>(&self, key: K) -> Option<&str>
	where K: values::AsHeaderName {
		self.values.get_str(key)
	}
}

impl Default for ResponseHeader {
	fn default() -> Self {
		Self {
			status_code: StatusCode::OK,
			content_type: ContentType::None,
			values: HeaderValues::new()
		}
	}
}