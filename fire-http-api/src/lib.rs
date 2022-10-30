#![doc = include_str!("../README.md")]

#[doc(hidden)]
#[macro_use]
pub mod util;
pub mod error;
mod request;
// #[cfg(feature = "stream")]
// pub mod stream;

pub use error::ApiError;
pub use request::{Request, Method};

#[doc(hidden)]
pub use fire;
#[doc(hidden)]
pub use serde_json;

pub use codegen::api;