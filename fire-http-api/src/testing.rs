use crate::error::ApiError;

use serde::de::DeserializeOwned;

use fire::{Data, Body, FirePit, Error};
use fire::header::{self, RequestHeader, HeaderValues, StatusCode, ContentType, Mime};


pub struct FirePitApi {
	inner: FirePit
}

impl FirePitApi {
	pub fn new(inner: FirePit) -> Self {
		Self { inner }
	}

	pub fn data(&self) -> &Data {
		self.inner.data()
	}

	/// Routes the request to normal routes and returns their result.
	/// 
	/// Useful for tests and niche applications.
	/// 
	/// Returns None if no route was found matching the request.
	pub async fn route(
		&self, req: &mut fire::Request
	) -> Option<Result<fire::Response, Error>> {
		self.inner.route(req).await
	}

	pub async fn request<R>(
		&self,
		req: &R
	) -> Result<R::Response, R::Error>
	where
		R: crate::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static
	{
		self.request_with_header(req, HeaderValues::new()).await
	}

	pub async fn request_with_header<R>(
		&self,
		req: &R,
		header: HeaderValues
	) -> Result<R::Response, R::Error>
	where
		R: crate::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static
	{
		let mut header = RequestHeader {
			address: "127.0.0.0:0".parse().unwrap(),
			method: R::METHOD,
			uri: R::PATH.parse().unwrap(),
			values: header
		};
		header.values.insert(
			header::CONTENT_TYPE,
			ContentType::from(Mime::JSON)
		);

		let mut req = fire::Request::new(header, Body::serialize(req).unwrap());

		let mut resp = self.route(&mut req).await.unwrap()
			.map_err(|e| R::Error::request(e.to_string()))?;

		if resp.header().status_code() != &StatusCode::OK {
			return Err(resp.body.deserialize().await.unwrap())
		}

		resp.take_body().deserialize().await
			.map_err(|e| R::Error::internal(e.to_string()))
	}
}