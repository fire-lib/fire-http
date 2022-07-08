
use crate::request::{HyperRequest, RequestBuilder};
use crate::util::convert_fire_res_to_hyper_res;
use crate::routes::Routes;

use std::sync::Arc;
use std::net::SocketAddr;
use std::convert::Infallible;
use std::time::Duration;

use tracing::error;

use http::header::StatusCode;
use http::response::Response;
use http::body::FireHttpBody;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
// same as page size
const DEFAULT_REQUEST_SIZE_LIMIT: usize = 4096;// 4kb


#[derive(Debug)]
pub struct RequestConfigs {
	pub timeout: Duration,
	pub size_limit: usize// in bytes
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

	/// panics if is 0
	pub fn size_limit(&mut self, size_limit: usize) {
		assert!(size_limit > 0, "size limit needs to be bigger than zero");
		self.size_limit = size_limit;
	}

}


// type that gets passet to the requests

pub type MoreWood<D> = Arc<Wood<D>>;

// IncredientsForAFire
pub struct Wood<D> {
	data: D,
	routes: Routes<D>,
	configs: RequestConfigs
}

impl<D> Wood<D> {

	pub fn new(data: D, routes: Routes<D>, configs: RequestConfigs) -> Self {
		Self { data, routes, configs }
	}

	pub fn routes(&self) -> &Routes<D> {
		&self.routes
	}

	pub fn data(&self) -> &D {
		&self.data
	}

	pub fn configs(&self) -> &RequestConfigs {
		&self.configs
	}

}

pub async fn new_spark<D>(
	wood: MoreWood<D>,
	hyper_req: HyperRequest,
	address: SocketAddr
) -> Result<hyper::Response<FireHttpBody>, Infallible> {
	let route_res = route(wood, hyper_req, address).await;
	let hyper_res = convert_fire_res_to_hyper_res( route_res );
	Ok(hyper_res)
}

pub async fn route<D>(
	wood: MoreWood<D>,
	hyper_req: HyperRequest,
	address: SocketAddr
) -> Response {
	// todo use a tracing span

	trace!("Request {} {}", hyper_req.method(), hyper_req.uri());

	let mut builder = RequestBuilder::new(hyper_req, address, wood.configs());

	// route raw_routes
	// response is Option<Response>
	let hyper_req = builder.hyper_ref().unwrap();
	let response = match wood.routes().route_raw(hyper_req) {
		Some(route) => {
			let res = route.call(&mut builder, wood.data()).await;
			match res {
				Some(Ok(res)) => Some(res),
				Some(Err(e)) => {
					error!("RawRoute returned an error {:?}", e);
					Some(e.status_code().into())
				},
				None => None
			}
		},
		None => None
	};

	let mut req = match builder.into_fire() {
		Ok(r) => r,
		Err(e) => {
			error!("Hyper Request Parsing error {:?}", e);
			return StatusCode::BadRequest.into()
		}
	};

	// Shadow Request

	let response = if let Some(res) = response {
		res
	} else {

		// first response
		let header = req.header();
		match wood.routes().route(header) {
			Some(route) => {
				let result = route.call( &mut req, wood.data() ).await;
				// is Ok(Response)
				match result {
					Ok(res) => res,
					Err(e) => {
						error!(
							"Route error: {:?} {:?}",
							req.header().uri().path(),
							e
						);
						e.status_code().into()
					}
				}
			},
			None => StatusCode::NotFound.into()
		}

	};

	// APPLY OVERRIDES

	// check with catcher
	#[allow(unused_assignments)]
	let req_header = req.header();
	let res_header = response.header();
	let resp = match wood.routes().route_catcher(req_header, res_header) {
		Some(route) => {
			route.call(req, response, wood.data()).await
				.unwrap_or_else(|e| e.status_code().into())
		},
		None => response
	};

	trace!("Response {:?}", resp);

	resp
}