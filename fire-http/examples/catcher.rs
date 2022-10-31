use fire_http as fire;

use fire::{Result, Request, Response, Data, get};
use fire::routes::Catcher;
use fire::header::{StatusCode, RequestHeader, ResponseHeader};
use fire::util::PinnedFuture;

#[get("/")]
fn hello_world() -> &'static str {
	"Hello, World!"
}

struct Error404Handler;

impl Catcher for Error404Handler {
	fn check(&self, _req: &RequestHeader, res: &ResponseHeader) -> bool {
		res.status_code() == &StatusCode::NOT_FOUND
	}

	fn call<'a>(
		&'a self,
		req: Request,
		mut res: Response,
		_data: &'a Data
	) -> PinnedFuture<'a, Result<Response>> {
		PinnedFuture::new(async move {
			let path = req.header().uri().path();
			let method = req.header().method();
			res.body = format!(
				"Error 404: Page \"{}\" With Method \"{}\" Not Found",
				path,
				method
			).into();

			Ok(res)
		})
	}
}


#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000").await
		.expect("Address could not be parsed");

	server.add_route(hello_world);
	server.add_catcher(Error404Handler);

	server.ignite().await.unwrap();
}