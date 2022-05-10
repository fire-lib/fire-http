
use crate::request::{Request, HyperRequest};
use crate::fire::RequestConfigs;

use std::task::Poll;
use std::pin::Pin;
use std::future::Future;
use std::task::Context;
use std::net::SocketAddr;

mod header;
use header::{ convert_hyper_parts_to_fire_header, convert_fire_header_to_hyper_header/* HyperHeaderMap*/ };
pub use header::HeaderError;

use http::Response;
use http::body::{FireHttpBody, BodyWithTimeout};


pub struct PinnedFuture<'a, O> {
	inner: Pin<Box<dyn Future<Output = O> + Send + 'a>>
}

impl<'a, O> PinnedFuture<'a, O> {
	pub fn new<F>(future: F) -> Self
	where F: Future<Output = O> + Send + 'a {
		Self {
			inner: Box::pin(future)
		}
	}
}

impl<O> Future for PinnedFuture<'_, O> {
	type Output = O;
	fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
		self.get_mut().inner.as_mut().poll(cx)
	}
}


// private stuff


pub(crate) fn convert_hyper_req_to_fire_req(
	hyper_req: HyperRequest,
	address: SocketAddr,
	configs: &RequestConfigs
) -> Result<Request, HeaderError> {

	let (parts, body) = hyper_req.into_parts();

	let body = BodyWithTimeout::from_hyper_body(
		body,
		configs.size_limit,
		configs.timeout
	);
	let header = convert_hyper_parts_to_fire_header(parts, address)?;

	Ok( Request::new(header, body) )
}


// // Response
pub(crate) fn convert_fire_res_to_hyper_res(response: Response) -> hyper::Response<FireHttpBody> {

	// debug_checks
	#[cfg(debug_assertions)]
	let _ = validate_content_length(&response);

	// get parts
	let (mut parts, _) = hyper::Response::builder()
		.body(()).unwrap()
		.into_parts();
	convert_fire_header_to_hyper_header( &mut parts, response.header );

	hyper::Response::from_parts(parts, response.body.into_http_body())
}


#[cfg(debug_assertions)]
fn validate_content_length(response: &Response) -> Option<()> {
	let len = response.header().value("content-length")?;

	let len: usize = len.parse().expect("content-length not a number");

	let body_len = response.body.len()?;

	assert_eq!(len, body_len);

	Some(())
}