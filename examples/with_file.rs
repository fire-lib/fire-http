use fire_http as fire;
use fire::get_with_file;

get_with_file!{ HelloWorld, "/" => "./examples/www/hello_world.html" }

#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000").await
		.expect("Address could not be parsed");

	server.add_route(HelloWorld);

	server.light().await.unwrap();
}