use fire::get;
use fire_http as fire;

#[get("/")]
fn hello_world() -> &'static str {
	"Hello, World!"
}

#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000")
		.await
		.expect("Address could not be parsed");

	server.add_route(hello_world);

	server.ignite().await.unwrap();
}
