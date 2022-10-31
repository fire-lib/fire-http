use crate::{Data, Request, Error};
use crate::server::HyperRequest;
use crate::util::{
	convert_hyper_req_to_fire_req, convert_fire_resp_to_hyper_resp
};
use crate::routes::Routes;

use std::net::SocketAddr;
use std::convert::Infallible;
use std::time::Duration;

use tracing::error;

use types::header::StatusCode;
use types::response::Response;
use types::body::BodyHttp;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
// same as page size
const DEFAULT_REQUEST_SIZE_LIMIT: usize = 4096;// 4kb

#[derive(Debug)]
pub(crate) struct RequestConfigs {
	pub timeout: Duration,
	// in bytes
	pub size_limit: usize
}

impl RequestConfigs {
	pub fn new() -> Self {
		Self {
			timeout: DEFAULT_REQUEST_TIMEOUT,
			size_limit: DEFAULT_REQUEST_SIZE_LIMIT
		}
	}

	pub fn timeout(&mut self, timeout: Duration) {
		self.timeout = timeout;
	}

	/// ## Panics
	/// if is 0
	pub fn size_limit(&mut self, size_limit: usize) {
		assert!(size_limit > 0, "size limit needs to be bigger than zero");
		self.size_limit = size_limit;
	}
}

// IncredientsForAFire
pub(crate) struct Wood {
	data: Data,
	routes: Routes,
	configs: RequestConfigs
}

impl Wood {
	pub fn new(data: Data, routes: Routes, configs: RequestConfigs) -> Self {
		Self { data, routes, configs }
	}

	pub fn routes(&self) -> &Routes {
		&self.routes
	}

	pub fn data(&self) -> &Data {
		&self.data
	}

	pub fn configs(&self) -> &RequestConfigs {
		&self.configs
	}
}

pub(crate) async fn route_hyper(
	wood: &Wood,
	hyper_req: HyperRequest,
	address: SocketAddr
) -> Result<hyper::Response<BodyHttp>, Infallible> {
	let route_resp = route_hyper_req(wood, hyper_req, address).await;
	let hyper_resp = convert_fire_resp_to_hyper_resp(route_resp);
	Ok(hyper_resp)
}

pub(crate) async fn route_hyper_req(
	wood: &Wood,
	mut hyper_req: HyperRequest,
	address: SocketAddr
) -> Response {
	// todo use a tracing span

	trace!("Request {} {}", hyper_req.method(), hyper_req.uri());

	// route raw_routes
	// response is Option<Response>
	let resp = if let Some(route) = wood.routes().route_raw(&hyper_req) {
		let res = route.call(&mut hyper_req, wood.data()).await;
		match res {
			Some(Ok(res)) => Some(res),
			Some(Err(e)) => {
				error!("RawRoute returned an error {:?}", e);
				Some(e.status_code().into())
			},
			None => None
		}
	} else {
		None
	};

	let req = convert_hyper_req_to_fire_req(hyper_req, address, wood.configs());
	let mut req = match req {
		Ok(r) => r,
		Err(e) => {
			if let Some(resp) = resp {
				return resp
			}
			error!("Could not parse the hyper request {:?}", e);
			return StatusCode::BAD_REQUEST.into()
		}
	};

	// normal route
	let resp = if let Some(r) = resp {
		r
	} else {
		match route(wood, &mut req).await {
			Some(Ok(resp)) => resp,
			Some(Err(e)) => {
				error!(
					"Route error: {:?} {:?}",
					req.header().uri().path(),
					e
				);
				e.status_code().into()
			},
			None => StatusCode::NOT_FOUND.into()
		}
	};

	// APPLY OVERRIDES

	// check with catcher
	let req_header = req.header();
	let res_header = resp.header();
	let resp = match wood.routes().route_catcher(req_header, res_header) {
		Some(route) => {
			route.call(req, resp, wood.data()).await
				.unwrap_or_else(|e| e.status_code().into())
		},
		None => resp
	};

	trace!("Response {:?}", resp);

	resp
}

pub(crate) async fn route(
	wood: &Wood,
	req: &mut Request
) -> Option<Result<Response, Error>> {
	// first response
	let r = wood.routes().route(req.header())?
		.call(req, wood.data()).await;

	Some(r)
}