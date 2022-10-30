use fire_api as api;

use std::fmt;

use api::error::{ApiError, Error as ErrorTrait, StatusCode};
use api::stream::{Stream, StreamKind, Streamer};

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
struct SenderReq {
	count: u64
}

#[derive(Debug, Serialize, Deserialize)]
struct SenderMsg {
	lucky_number: u64
}

impl Stream for SenderReq {
	type Message = SenderMsg;
	type Error = Error;

	const KIND: StreamKind = StreamKind::Sender;
	const ACTION: &'static str = "Hi";
}

#[fire::api_stream(SenderReq)]
async fn lucky_number_stream(
	req: SenderReq,
	mut streamer: Streamer<SenderMsg>
) -> Result<(), Error> {
	for i in 0..req.count {
		streamer.send(SenderMsg {
			lucky_number: i
		}).await
			.map_err(|e| Error::Internal(e.to_string()))?;
	}

	Ok(())
}