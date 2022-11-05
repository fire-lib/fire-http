use fire_api as api;

use std::fmt;

use api::{Method, Request};
use api::error::{ApiError, Error as ErrorTrait, StatusCode};

use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize)]
pub enum Error {
	Internal(String),
	Request(String)
}

impl ApiError for Error {
	fn internal<E: ErrorTrait>(e: E) -> Self {
		Self::Internal(e.to_string())
	}

	fn request<E: ErrorTrait>(e: E) -> Self {
		Self::Request(e.to_string())
	}

	fn status_code(&self) -> StatusCode {
		match self {
			Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::Request(_) => StatusCode::BAD_REQUEST
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NameReq;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Name {
	firstname: String,
	lastname: String
}

impl Request for NameReq {
	type Response = Name;
	type Error = Error;

	const PATH: &'static str = "/name";
	const METHOD: Method = Method::GET;
}

#[fire::api(NameReq)]
fn get_name(
	_req: NameReq,
	_some_data: &Name,
	_more_data: &NameReq
) -> Result<Name, Error> {
	Ok(Name {
		firstname: "Albert".into(),
		lastname: "Einstein".into()
	})
}