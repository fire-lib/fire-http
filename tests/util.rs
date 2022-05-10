#![allow(dead_code, unused_macros)]

use fire_http as fire;

use fire::http::Body;

macro_rules! spawn_server {
	(|$builder:ident| $block:block) => (
		spawn_server!(|$builder| $block, ())
	);
	(|$builder:ident| $block:block, $data:expr) => ({
		use std::net::{SocketAddr, Ipv4Addr};

		let socket_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0);
		let mut $builder = fire::build(socket_addr, $data).unwrap();
		let _ = $block;
		let (addr, fut) = $builder.test_light();
		tokio::spawn(async move {
			fut.await;
		});

		addr
	})
}

// returns a 
macro_rules! make_request {
	($method:expr, $srv_addr:expr, $uri:expr, |$builder:ident| $block:block) => (async {
		let $builder = hyper::Request::builder()
			.method($method)
			.uri(format!("http://{}{}", $srv_addr, $uri));
		let request: hyper::Request<_> = $block;
		// build client
		let hyper_res = hyper::Client::new()
			.request(request).await
			.expect("hyper request error")
			// now we need to convert the body into our body
			.map(fire::http::Body::from_hyper_body);

		util::TestResponse::new(hyper_res)
	});
	($method:expr, $srv_addr:expr, $uri:expr, $body:expr) => (
		make_request!($method, $srv_addr, $uri, |builder| {
			builder.body($body.into()).expect("could not build request")
		})
	);
	($method:expr, $srv_addr:expr, $uri:expr) => (
		make_request!($method, $srv_addr, $uri, hyper::Body::empty())
	);
}

#[derive(Debug)]
pub struct TestResponse {
	inner: hyper::Response<Body>
}

impl TestResponse {

	pub fn new(inner: hyper::Response<Body>) -> Self {
		Self {inner}
	}

	pub fn assert_status(self, other: u16) -> Self {
		assert_eq!(self.inner.status().as_u16(), other, "status code doens't match");
		self
	}

	pub fn assert_header(self, key: &str, value: impl AsRef<str>) -> Self {
		let v = self.inner.headers()
			.get(key).expect(&format!("header with key {:?} not found", key))
			.to_str().expect("header does not only contain visible ASCII chars");
		assert_eq!(v, value.as_ref(), "value does not match");
		self
	}

	pub fn assert_not_header(self, key: &str) -> Self {
		if self.inner.headers().get(key).is_some() {
			panic!("expected no header named {}", key);
		}
		self
	}

	pub fn header(&self, key: &str) -> Option<&str> {
		self.inner.headers().get(key)
			.and_then(|v| v.to_str().ok())
	}

	pub async fn assert_body_str(mut self, value: &str) -> Self {
		let body = self.inner.body_mut().take().into_string().await
			.expect("could not convert response body to string");
		assert_eq!(body, value, "body does not match value");
		self
	}

	pub async fn assert_body_vec(mut self, value: &[u8]) -> Self {
		let body = self.inner.body_mut().take().into_vec().await
			.expect("could not convert response body to vec");
		assert_eq!(body, value, "body does not match value");
		self
	}
}