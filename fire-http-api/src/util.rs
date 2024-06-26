use crate::error::Error;
use crate::response::ResponseSettings;
use crate::ApiError;
use crate::Request;

use std::time::Duration;

use fire::extractor::ExtractorError;
use representation::request::SerializeError;

use fire::error::ServerErrorKind;
use fire::header::{HeaderValues, Method, Mime};
use fire::{Body, Response};
use tracing::info;

pub fn setup_request<R: Request>(
	req: &mut fire::Request,
) -> Result<(), R::Error> {
	req.set_size_limit(Some(R::SIZE_LIMIT));
	req.set_timeout(Some(Duration::from_secs(R::TIMEOUT as u64)));

	// check headers
	let headers_missing = R::HEADERS
		.iter()
		.any(|key| req.header().value(*key).is_none());
	if headers_missing {
		return Err(R::Error::from_error(Error::HeadersMissing(R::HEADERS)));
	}

	Ok(())
}

pub async fn deserialize_req<R: Request + Send + 'static>(
	req: &mut fire::Request,
) -> Result<R, R::Error> {
	// since a get request does not have a body let's parse the query parameters
	if R::METHOD == Method::GET {
		req.deserialize_query()
			.map_err(|e| R::Error::from_error(Error::Deserialize(e)))
	} else {
		req.deserialize()
			.await
			.map_err(|e| R::Error::from_error(Error::Deserialize(e)))
	}
}

pub fn extraction_error<R: Request>(e: impl ExtractorError) -> R::Error {
	R::Error::from_error(Error::ExtractionError(e.into_std()))
}

pub fn serialize_resp<R: Request>(
	resp: &R::Response,
) -> Result<Body, R::Error> {
	Body::serialize(resp).map_err(|e| {
		R::Error::from_error(Error::Serialize(SerializeError::Json(e)))
	})
}

/// todo find a better name
pub fn transform_body_to_response<R: Request>(
	res: Result<(ResponseSettings, Body), R::Error>,
) -> fire::Result<Response> {
	let (status, headers, body) = match res {
		Ok((settings, body)) => (settings.status, settings.headers, body),
		Err(e) => {
			info!(error = ?e, "api response error");
			// status code should define the error and this should referenced to it

			let body = Body::serialize(&e).map_err(|e| {
				fire::Error::new(ServerErrorKind::InternalServerError, e)
			})?;

			(e.status_code(), HeaderValues::new(), body)
		}
	};

	let mut resp = Response::builder()
		.status_code(status)
		.content_type(Mime::JSON)
		.body(body);
	*resp.values_mut() = headers;

	Ok(resp.build())
}

pub fn validate_request<R: Request>(ty: &str) {
	// if it is a get request, make sure the type cannot be parsed from a a null value
	// since that would mean no request can go through
	if R::METHOD != Method::GET {
		return;
	}

	let r = serde_json::from_value::<R>(serde_json::Value::Null);
	if r.is_ok() {
		panic!(
			"Get request cannot be parsed from a null value, \
		make sure the type {ty} is not a unit struct"
		);
	}
}
