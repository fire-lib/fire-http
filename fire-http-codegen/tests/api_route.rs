use fire::impl_res_extractor;
use fire_api as api;

use std::fmt;

use api::error::{self, Error as ApiError, StatusCode};
use api::{Method, Request};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub enum Error {
	Internal(String),
	Request(String),
}

impl error::ApiError for Error {
	fn from_error(e: ApiError) -> Self {
		match e {
			ApiError::HeadersMissing(_) | ApiError::Deserialize(_) => {
				Self::Request(e.to_string())
			}
			e => Self::Internal(e.to_string()),
		}
	}

	fn status_code(&self) -> StatusCode {
		match self {
			Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::Request(_) => StatusCode::BAD_REQUEST,
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
	lastname: String,
}

impl_res_extractor!(Name);

impl Request for NameReq {
	type Response = Name;
	type Error = Error;

	const PATH: &'static str = "/name";
	const METHOD: Method = Method::GET;
}

#[fire::api(NameReq)]
async fn get_name(_req: NameReq, _some_data: &Name) -> Result<Name, Error> {
	Ok(Name {
		firstname: "Albert".into(),
		lastname: "Einstein".into(),
	})
}
