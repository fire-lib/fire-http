
use fire_http as fire;
use fire::get;

// Default Data
pub struct Data;


get!{ HelloWorld<()>, "/", |_r| -> &'static str {
	"Hello, World!"
} }

#[tokio::main]
async fn main() {

	let mut server = fire::build("0.0.0.0:3000", ())
		.expect("Address could not be parsed");

	server.add_route(HelloWorld);

	server.light().await.unwrap();
	
}