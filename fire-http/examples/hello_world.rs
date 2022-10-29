use fire_http as fire;

use fire::get;


get!{ HelloWorld, "/",
	|_r| -> &'static str {
		"Hello, World!"
	}
}

#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000").await
		.expect("Address could not be parsed");

	server.add_route(HelloWorld);

	server.light().await.unwrap();
}