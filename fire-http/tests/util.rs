#![allow(dead_code, unused_macros)]

use fire::Body;
use fire_http as fire;
use types::body::BodyHttp;

use std::io;

macro_rules! spawn_server {
	(|$builder:ident| $block:block) => {{
		use std::net::{Ipv4Addr, SocketAddr};

		let socket_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0);
		let mut $builder = fire::build(socket_addr).await.unwrap();
		let _ = $block;
		let fire = $builder.build().await.unwrap();
		let addr = fire.local_addr().unwrap();
		tokio::task::spawn(fire.ignite());

		addr
	}};
}

macro_rules! other_err {
	($e:expr) => {
		io::Error::new(io::ErrorKind::Other, $e)
	};
}

pub async fn send_request(
	req: hyper::Request<BodyHttp>,
) -> io::Result<hyper::Response<hyper::Body>> {
	let client = hyper::Client::builder().build_http();

	client.request(req).await.map_err(|e| other_err!(e))
}

macro_rules! make_request {
	(
		$method:expr, $srv_addr:expr, $uri:expr,
		|$builder:ident| $block:block
	) => {
		async {
			let addr = $srv_addr.to_string();
			let uri = format!("http://{addr}{}", $uri);
			let $builder = hyper::Request::builder()
				.method($method)
				.uri(uri)
				.header("host", &addr);
			let resp = util::send_request($block)
				.await
				.expect("failed to send request")
				.map(fire::Body::from_hyper);

			util::TestResponse::new(resp)
		}
	};
	($method:expr, $srv_addr:expr, $uri:expr, $body:expr) => {
		make_request!($method, $srv_addr, $uri, |builder| {
			builder
				.body(fire::Body::into_http_body($body.into()))
				.expect("could not build request")
		})
	};
	($method:expr, $srv_addr:expr, $uri:expr) => {
		make_request!($method, $srv_addr, $uri, fire::Body::new())
	};
}

#[derive(Debug)]
pub struct TestResponse {
	inner: hyper::Response<Body>,
}

impl TestResponse {
	pub fn new(inner: hyper::Response<Body>) -> Self {
		Self { inner }
	}

	pub fn assert_status(self, other: u16) -> Self {
		assert_eq!(
			self.inner.status().as_u16(),
			other,
			"status code doens't match"
		);
		self
	}

	pub fn assert_header(self, key: &str, value: impl AsRef<str>) -> Self {
		let v = self
			.inner
			.headers()
			.get(key)
			.expect(&format!("header with key {:?} not found", key))
			.to_str()
			.expect("header does not only contain visible ASCII chars");
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
		self.inner.headers().get(key).and_then(|v| v.to_str().ok())
	}

	pub async fn assert_body_str(mut self, value: &str) -> Self {
		let body = self
			.inner
			.body_mut()
			.take()
			.into_string()
			.await
			.expect("could not convert response body to string");
		assert_eq!(body, value, "body does not match value");
		self
	}

	pub async fn assert_body_vec(mut self, value: &[u8]) -> Self {
		let body = self
			.inner
			.body_mut()
			.take()
			.into_bytes()
			.await
			.expect("could not convert response body to vec");
		assert_eq!(body, value, "body does not match value");
		self
	}
}
