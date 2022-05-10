
mod raw_route;
pub use raw_route::RawRoute;

mod route;
pub use route::{Route, check_static};

mod catcher;
pub use catcher::Catcher;

use crate::request::HyperRequest;

use http::header::{ RequestHeader, ResponseHeader };


type BoxedRawRoute<D> = Box<dyn RawRoute<D>>;
type BoxedRoute<D> = Box<dyn Route<D>>;
type BoxedCatcher<D> = Box<dyn Catcher<D>>;


pub struct Routes<D> {
	// maybe store static routes in hashmap??
	raw: Vec<BoxedRawRoute<D>>,
	basic: Vec<BoxedRoute<D>>,
	catcher: Vec<BoxedCatcher<D>>
}

impl<D> Routes<D> {

	pub fn new() -> Self {
		Self{
			raw: vec![],
			basic: vec![],
			catcher: vec![]
		}
	}

	pub fn push_raw<R>(&mut self, route: R)
	where R: RawRoute<D> + 'static {
		self.raw.push(Box::new(route))
	}

	pub fn push<R>(&mut self, route: R)
	where R: Route<D> + 'static {
		self.basic.push(Box::new(route))
	}

	pub fn push_catcher<C>(&mut self, catcher: C)
	where C: Catcher<D> + 'static {
		self.catcher.push(Box::new(catcher))
	}

	pub fn route_raw(
		&self,
		hyper_request: &HyperRequest
	) -> Option<&BoxedRawRoute<D>> {
		for route in &self.raw {
			if route.check(hyper_request) {
				return Some(route)
			}
		}
		None
	}

	pub fn route(
		&self,
		request_header: &RequestHeader
	) -> Option<&BoxedRoute<D>> {
		for route in &self.basic {
			if route.check( request_header ) {
				return Some( route )
			}
		}
		None
	}

	pub fn route_catcher(
		&self,
		request_header: &RequestHeader,
		response_header: &ResponseHeader
	) -> Option<&BoxedCatcher<D>> {
		for catcher in &self.catcher {
			if catcher.check(request_header, response_header) {
				return Some(catcher)
			}
		}
		None
	}

}