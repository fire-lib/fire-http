//! Http types for the fire http crate.
//!
//! At the moment these types are more suitable for server implementations than
//! for clients.
//!
//! The `reqwest` crate is great and should be sufficient for almost all client
//! needs.
//!
//! ## Features
//!
//! ### hyper_body
//! Adds support for the `hyper::Body` type in `Body`.
//!
//! ### json
//! Adds json serialization and deserialization support for
//! the `Body` type and for `HeaderValues`.
//!
//! ### timeout
//! Adds the `BodyTimeout` type, allowing to set a timeout
//! for reading from the body.
//!

/// Reexport the http crate
pub use http;

pub mod header;
pub mod body;
pub use body::Body;

pub mod request;
pub use request::Request;

pub mod response;
pub use response::Response;