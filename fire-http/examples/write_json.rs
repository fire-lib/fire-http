use fire_http as fire;
use fire::get_json;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct MyType {
	crazy: &'static str,
	good: &'static str
}

#[get_json("/")]
fn hello_world() -> MyType {
	MyType {
		crazy: "crazy",
		good: "good"
	}
}

#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000").await
		.expect("Address could not be parsed");

	server.add_route(hello_world);

	server.light().await.unwrap();
}