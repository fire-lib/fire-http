use fire_http as fire;

use fire::header::{
	Method, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
	ACCESS_CONTROL_ALLOW_ORIGIN, X_XSS_PROTECTION,
};
use fire::header::{RequestHeader, ResponseHeader, StatusCode};
use fire::routes::Catcher;
use fire::util::PinnedFuture;
use fire::{get, Data, Request, Response};

#[get("/")]
fn hello_world() -> &'static str {
	"Hello, World!"
}

struct CorsHeaders;

impl Catcher for CorsHeaders {
	fn check(&self, _req: &RequestHeader, _res: &ResponseHeader) -> bool {
		true
	}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		res: &'a mut Response,
		_data: &'a Data,
	) -> PinnedFuture<'a, fire::Result<()>> {
		let values = &mut res.header.values;

		// if we have a options request this means we need to
		// answer with access-control-allow-origin
		if req.header().method == Method::OPTIONS {
			res.header.status_code = StatusCode::NO_CONTENT;
			values.insert(ACCESS_CONTROL_ALLOW_METHODS, "POST, PUT");
		}

		values.insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*");
		values.insert(ACCESS_CONTROL_ALLOW_HEADERS, "content-type");
		values.insert(X_XSS_PROTECTION, "0");

		PinnedFuture::new(async move { Ok(()) })
	}
}

#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000")
		.await
		.expect("Address could not be parsed");

	server.add_route(hello_world);
	server.add_catcher(CorsHeaders);

	server.ignite().await.unwrap();
}
