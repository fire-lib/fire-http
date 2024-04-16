use fire_http as fire;

use fire::extractor::Res;
use fire::header::Mime;
use fire::{get, post, Error, Request, Response, Result};

use std::sync::Mutex;

struct LastPost(Mutex<String>);

#[get("/")]
fn hello_world(last_post: Res<LastPost>) -> Response {
	let body = {
		let last_post = last_post.0.lock().unwrap();
		format!(
			"Hello, World! Post Something:<br>
		<form method=\"POST\">
			<input type=\"text\" name=\"text\" placeholder=\"Something\">
		</form>
		<h3>Last Post</h3>
		<p>{}</p>",
			&last_post
		)
	};

	Response::builder()
		.content_type(Mime::HTML)
		.body(body)
		.build()
}

#[post("/")]
async fn hello_world_post(
	req: &mut Request,
	last_post: Res<'_, LastPost>,
) -> Result<String> {
	// we need to update the size limit
	req.set_size_limit(Some(256));

	let body = req
		.take_body()
		.into_string()
		.await
		.map_err(Error::from_client_io)?;

	let res = format!("Posted Body: {}", body);

	*last_post.0.lock().unwrap() = body;

	Ok(res)
}

#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000")
		.await
		.expect("Address could not be parsed");

	server.add_data(LastPost(Mutex::new(String::new())));

	server.request_size_limit(1);
	server.add_route(hello_world);
	server.add_route(hello_world_post);

	server.ignite().await.unwrap();
}
