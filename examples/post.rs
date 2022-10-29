use fire_http as fire;

use fire::{Result, Error, Response, get, post};
use fire::header::Mime;

use std::sync::Mutex;


struct LastPost(Mutex<String>);

get!{ HelloWorld, "/",
	|_r, last_post: LastPost| -> Response {
		let body = {
			let last_post = last_post.0.lock().unwrap();
			format!("Hello, World! Post Something:<br>
			<form method=\"POST\">
				<input type=\"text\" name=\"text\" placeholder=\"Something\">
			</form>
			<h3>Last Post</h3>
			<p>{}</p>", &last_post)
		};

		Response::builder()
			.content_type(Mime::HTML)
			.body(body)
			.build()
	}
}

post!{ HelloWorldPost, "/",
	|req, last_post: LastPost| -> Result<String> {
		// we need to update the size limit
		req.set_size_limit(Some(256));

		let body = req.take_body().into_string().await
			.map_err(Error::from_client_io)?;

		let res = format!("Posted Body: {}", body);

		*last_post.0.lock().unwrap() = body;

		Ok(res)
	}
}


#[tokio::main]
async fn main() {

	let last_post = LastPost(Mutex::new(String::new()));

	let mut server = fire::build("0.0.0.0:3000").await
		.expect("Address could not be parsed");

	server.add_data(last_post);

	server.request_size_limit(1);
	server.add_route(HelloWorld);
	server.add_route(HelloWorldPost);

	server.light().await
		.unwrap();
	
}