// todo if this should ever be used outside of testing the uri parameter
// should be replaced with a hashmap or something simiar

use crate::error::ApiError;

use fire::routes::ParamsNames;
use serde::de::DeserializeOwned;

use fire::header::{HeaderValues, Mime, StatusCode};
use fire::{Body, Data, Error, FirePit, Request, Response};

pub struct FirePitApi {
	inner: FirePit,
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
		req: &mut Request,
	) -> Option<Result<Response, Error>> {
		self.inner.route(req).await
	}

	pub async fn request<R>(&self, req: &R) -> Result<R::Response, R::Error>
	where
		R: crate::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static,
	{
		let params = ParamsNames::parse(R::PATH);
		assert!(params.is_empty(), "path parameters are not allowed");

		self.request_with_header(R::PATH, req, HeaderValues::new())
			.await
	}

	pub async fn request_with_uri<R>(
		&self,
		uri: impl AsRef<str>,
		req: &R,
	) -> Result<R::Response, R::Error>
	where
		R: crate::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static,
	{
		self.request_with_header(uri.as_ref(), req, HeaderValues::new())
			.await
	}

	pub async fn request_with_header<R>(
		&self,
		uri: impl AsRef<str>,
		req: &R,
		header: HeaderValues,
	) -> Result<R::Response, R::Error>
	where
		R: crate::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static,
	{
		let mut raw_req =
			Request::builder(uri.as_ref().parse().unwrap()).method(R::METHOD);
		*raw_req.values_mut() = header;
		let mut req = raw_req
			.content_type(Mime::JSON)
			.body(Body::serialize(req).unwrap())
			.build();

		self.request_raw::<R>(&mut req).await
	}

	pub async fn request_raw<R>(
		&self,
		req: &mut Request,
	) -> Result<R::Response, R::Error>
	where
		R: crate::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static,
	{
		let mut resp = self
			.route(req)
			.await
			.unwrap()
			.map_err(|e| R::Error::request(e.to_string()))?;

		if resp.header().status_code() != &StatusCode::OK {
			return Err(resp.body.deserialize().await.unwrap());
		}

		resp.take_body()
			.deserialize()
			.await
			.map_err(|e| R::Error::internal(e.to_string()))
	}
}
