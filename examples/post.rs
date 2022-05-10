
use fire_http as fire;

use fire::Result;
use fire::data_struct;
use fire::error::Error;
use fire::{ get, post };
use fire::http::header::Mime;
use fire::http::response::Response;

use std::sync::Mutex;

data_struct! {
	/// Default Data
	struct Data {
		last_post: Mutex<String>
	}
}


get!{ HelloWorld, "/", |_r, last_post| -> Response {
	let body = {
		let last_post = last_post.lock().unwrap();
		format!("Hello, World! Post Something:<br>
		<form method=\"POST\">
			<input type=\"text\" name=\"text\" placeholder=\"Something\">
		</form>
		<h3>Last Post</h3>
		<p>{}</p>", &last_post)
	};

	Response::builder()
		.content_type( Mime::Html )
		.body(body)
		.build()
} }

post!{ HelloWorldPost, "/", |req, last_post| -> Result<String> {
	// we need to update the max_size
	// else it won't work
	req.set_size_limit(256);

	let body = req.take_body().into_string().await
		.map_err(Error::from_client_io)?;

	let res = format!("Posted Body: {}", body);

	*last_post.lock().unwrap() = body;
	//let body = req.header().host();
	Ok(res)
} }


#[tokio::main]
async fn main() {

	let data = Data {
		last_post: Mutex::new(String::new())
	};

	let mut server = fire::build( "0.0.0.0:3000", data )
		.expect("Address could not be parsed");

	server.request_size_limit(1);
	server.add_route( HelloWorld );
	server.add_route( HelloWorldPost );

	server.light().await.unwrap();
	
}