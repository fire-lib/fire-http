
use fire_http as fire;
use fire::json_get;
use serde::{ Serialize, Deserialize };

// Default Data
pub struct Data;

#[derive(Serialize, Deserialize)]
pub struct MyType {
	crazy: &'static str,
	good: &'static str
}

json_get!{ HelloWorld, "/", |_r| -> MyType {
	MyType {
		crazy: "crazy",
		good: "good"
	}
} }

#[tokio::main]
async fn main() {

	let mut server = fire::build( "0.0.0.0:3000", Data )
		.expect("Address could not be parsed");

	server.add_route( HelloWorld );

	server.light().await.unwrap();
	
}