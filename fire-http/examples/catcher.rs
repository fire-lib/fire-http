use fire_http as fire;

use fire::{get, catcher, Response};
use fire::header::StatusCode;

get!{ HelloWorld, "/",
	|_r| -> &'static str {
		"Hello, World!"
	}
}

catcher!{ TheCatcher,
	|_r, header| {
		header.status_code() == &StatusCode::NOT_FOUND
	},
	|req, res| -> Response {
		let path = req.header().uri().path();
		let method = req.header().method();
		res.body = format!(
			"Error 404: Page \"{}\" With Method \"{}\" Not Found",
			path,
			method
		).into();
		res
	}
}


#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000").await
		.expect("Address could not be parsed");

	server.add_route(HelloWorld);
	server.add_catcher(TheCatcher);

	server.light().await.unwrap();
}