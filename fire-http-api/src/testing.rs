use crate::error::ApiError;

use serde::de::DeserializeOwned;

use fire::{Data, Body, FirePit, Error, Request, Response};
use fire::header::{HeaderValues, StatusCode, Mime};


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
		&self,
		req: &mut Request
	) -> Option<Result<Response, Error>> {
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
		let mut raw_req = Request::builder(R::PATH.parse().unwrap())
			.method(R::METHOD);
		*raw_req.values_mut() = header;
		let mut req = raw_req.content_type(Mime::JSON)
			.body(Body::serialize(req).unwrap())
			.build();

		self.request_raw::<R>(&mut req).await
	}

	pub async fn request_raw<R>(
		&self,
		req: &mut Request
	) -> Result<R::Response, R::Error>
	where
		R: crate::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static
	{
		let mut resp = self.route(req).await.unwrap()
			.map_err(|e| R::Error::request(e.to_string()))?;

		if resp.header().status_code() != &StatusCode::OK {
			return Err(resp.body.deserialize().await.unwrap())
		}

		resp.take_body().deserialize().await
			.map_err(|e| R::Error::internal(e.to_string()))
	}
}