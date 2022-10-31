use fire_http as fire;
use fire::fs::StaticFiles;

const CSS: StaticFiles = StaticFiles::new("/css", "./examples/www/css");


#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000").await
		.expect("Address could not be parsed");

	server.add_route(CSS);

	server.ignite().await.unwrap();
	
}