use std::fmt::{Debug, Display};

pub use fire::header::StatusCode;

use serde::Serialize;

/// Basic error trait which implements Debug + Display + Serialize
/// 
/// The usefulness is still undecided
pub trait Error: Debug + Display + Serialize {}

impl Error for &'static str {}
impl Error for String {}

/// The error that is sent if something goes wrong while responding
/// to a request.
/// ## Panics
/// If deserialization or serialization failes
/// this will result in a panic
pub trait ApiError: Debug + Display + Serialize {
	// server internal
	fn internal<E: Error>(error: E) -> Self;

	// an error with the request
	fn request<E: Error>(error: E) -> Self;

	fn status_code(&self) -> StatusCode;
}