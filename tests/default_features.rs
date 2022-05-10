
use fire_http as fire;

use fire::http::{Response, Body};
use fire::http::header::{StatusCode, Mime};

type Data = ();

#[macro_use]
mod util;


#[tokio::test]
async fn hello_world() {

	const BODY: &str = "Hello, World!";

	// build route
	fire::get!(HelloWorld, "/", |_r| -> &'static str {
		BODY
	});

	let addr = spawn_server!(|builder| {
		builder.add_route(HelloWorld);
	});

	// now do a request
	make_request!("GET", addr, "/").await
		.assert_status(200)
		.assert_header("content-type", "text/plain; charset=utf-8")
		.assert_header("content-length", BODY.len().to_string())
		.assert_body_str(BODY).await;

}

#[tokio::test]
async fn test_post() {

	const BODY: &str = "Hello, World!";

	// build route
	fire::post!(Post, "/", |req| -> Body {
		// we loose the timeout here
		req.take_body().into_body()
	});

	let addr = spawn_server!(|builder| {
		builder.add_route(Post);
	});

	// now do a request
	make_request!("POST", addr, "/", BODY).await
		.assert_status(200)
		// this is the default content-type
		// we should probably change that
		.assert_not_header("content-type")
		// because we return a stream
		// we don't know how big it is
		.assert_not_header("content-length")
		.assert_body_str(BODY).await;

}

#[tokio::test]
async fn test_catcher() {

	const BODY: &str = "Body not Found";

	// build route
	fire::catcher!(NotFound,
		|_r, header| {
			header.status_code() == &StatusCode::NotFound
		},
		|_r, res| -> Response {
			Response::builder()
				.status_code(*res.header().status_code())
				.content_type(Mime::Text)
				.body(BODY)
				.build()
		}
	);

	let addr = spawn_server!(|builder| {
		builder.add_catcher(NotFound);
	});

	// now do a request
	make_request!("GET", addr, "/").await
		.assert_status(404)
		// this is the default content-type
		// we should probably change that
		.assert_header("content-type", "text/plain; charset=utf-8")
		.assert_header("content-length", BODY.len().to_string())
		.assert_body_str(BODY).await;

}

#[tokio::test]
async fn anything() {

	struct Data(Vec<u8>);
	impl Data {
		fn data(&self) -> &[u8] {
			&self.0
		}
	}

	// some random data
	let mut data = vec![];
	for i in 1..=1024 {
		data.push((i % 255) as u8);
	}

	// build route
	fire::get!(Get, "/", |_r, data| -> Vec<u8> {
		data.to_vec()
	});

	let addr = spawn_server!(
		|builder| {
			builder.add_route(Get);
		},
		Data(data.clone())
	);

	// now do a request
	make_request!("GET", addr, "/").await
		.assert_status(200)
		// this is the default content-type
		// we should probably change that
		.assert_header("content-type", "application/octet-stream")
		.assert_header("content-length", data.len().to_string())
		.assert_body_vec(&data).await;

}