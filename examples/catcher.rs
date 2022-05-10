
use fire_http as fire;
use fire::{ get, catcher };
use fire::http::header::StatusCode;
use fire::http::Response;

get!{ HelloWorld<()>, "/", |_r| -> &'static str {
	"Hello, World!"
} }

catcher!{ TheCatcher<()>,
	|_r, header| {
		header.status_code() == &StatusCode::NotFound
	},
	|req, res| -> Response {
		let path = req.header().uri().path();
		let method = req.header().method();
		res.body = format!( "Error 404: Page \"{}\" With Method \"{}\" Not Found", path, method ).into();
		res
	}
}


#[tokio::main]
async fn main() {

	let mut server = fire::build("0.0.0.0:3000", ())
		.expect("Address could not be parsed");

	server.add_route( HelloWorld );
	server.add_catcher( TheCatcher );

	server.light().await.unwrap();
}