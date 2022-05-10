
use std::{fmt, error, io};
use error::Error as ErrorTrait;

use http::header::StatusCode;
use http::bytes_stream::SizeLimitReached;


pub type Result<T> = std::result::Result<T, Error>;


/// A universal error type which contains a source and a kind.
/// 
/// An error is either associated with the client or the server.
#[derive(Debug)]
pub struct Error {
	kind: ErrorKind,
	source: Option<Box<dyn ErrorTrait + Send + Sync>>
}

impl Error {
	/// Creates a new error.
	pub fn new<K, E>(kind: K, error: E) -> Self
	where
		K: Into<ErrorKind>,
		E: Into<Box<dyn ErrorTrait + Send + Sync>> {
		Self {
			kind: kind.into(),
			source: Some(error.into())
		}
	}

	/// Creates a new error without a source.
	pub fn empty<K>(kind: K) -> Self
	where K: Into<ErrorKind> {
		Self {
			kind: kind.into(),
			source: None
		}
	}

	/// Returns the `StatusCode` corresponding to the `ErrorKind`.
	pub fn status_code(&self) -> StatusCode {
		match self.kind {
			ErrorKind::Client(c) => c.into(),
			ErrorKind::Server(s) => s.into()
		}
	}

	/// Returns a new error from an io::Error originated from the client.
	pub fn from_client_io(error: io::Error) -> Self {
		// try to detect if source is known to us
		Self::new(
			ClientErrorKind::from_io(&error),
			error
		)
	}

	/// Returns a new error originating from the server.
	pub fn from_server_error<E>(error: E) -> Self
	where E: Into<Box<dyn ErrorTrait + Send + Sync>> {
		Self::new(ServerErrorKind::InternalServerError, error)
	}
}

impl<T> From<T> for Error
where T: Into<ErrorKind> {
	fn from(e: T) -> Self {
		Self::empty(e)
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

impl error::Error for Error {
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		self.source.as_ref().map(|e| e.source()).flatten()
	}
}

/// An error can either come from the client or the server.
#[derive(Debug)]
pub enum ErrorKind {
	Client(ClientErrorKind),
	Server(ServerErrorKind)
}


impl From<ClientErrorKind> for ErrorKind {
	fn from(k: ClientErrorKind) -> Self {
		Self::Client(k)
	}
}

impl From<ServerErrorKind> for ErrorKind {
	fn from(k: ServerErrorKind) -> Self {
		Self::Server(k)
	}
}


macro_rules! error_kind {
	($name:ident, $($kind:ident),*) => (
		#[derive(Debug, Clone, Copy, PartialEq, Eq)]
		pub enum $name {
			$($kind),*
		}

		impl From<$name> for StatusCode {
			fn from(k: $name) -> Self {
				match k {
					$($name::$kind => Self::$kind),*
				}
			}
		}
	)
}

// impl ClientErrorKind
error_kind!( ClientErrorKind,
	BadRequest,
	Unauthorized,
	PaymentRequired,
	Forbidden,
	NotFound,
	MethodNotAllowed,
	NotAcceptable,
	ProxyAuthenticationRequired,
	RequestTimeout,
	Conflict,
	Gone,
	LengthRequired,
	PreconditionFailed,
	RequestEntityTooLarge,
	RequestURITooLarge,
	UnsupportedMediaType,
	RequestedRangeNotSatisfiable,
	ExpectationFailed
);

impl ClientErrorKind {
	/// Converts an io::Error into the appropriate kind.
	pub fn from_io(error: &io::Error) -> Self {
		// first try to detect the source
		if let Some(src) = error.get_ref() {
			if src.is::<SizeLimitReached>() {
				return Self::RequestEntityTooLarge
			}
		}

		use io::ErrorKind::*;
		// try to detect if source is known to us
		match error.kind() {
			NotFound => Self::NotFound,
			PermissionDenied => Self::Unauthorized,
			// this should probably not happen?
			AlreadyExists => Self::Conflict,
			InvalidInput |
			InvalidData |
			Other => Self::BadRequest,
			TimedOut => Self::RequestTimeout,
			_ => Self::ExpectationFailed
		}
	}
}

// Impl ServerErrorKind
error_kind!( ServerErrorKind,
	InternalServerError,
	NotImplemented,
	BadGateway,
	ServiceUnavailable,
	GatewayTimeout
);