#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// Reexport the http crate
pub use http;

pub mod header;
pub mod body;
pub use body::Body;

pub mod request;
pub use request::Request;

pub mod response;
pub use response::Response;