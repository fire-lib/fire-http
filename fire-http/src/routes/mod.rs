mod raw_route;
pub use raw_route::{RawRoute, HyperRequest};

mod route;
pub use route::{Route, check_static};

mod catcher;
pub use catcher::Catcher;

pub mod util;

use crate::header::RequestHeader;

use std::slice;


type BoxedRawRoute = Box<dyn RawRoute>;
type BoxedRoute = Box<dyn Route>;
type BoxedCatcher = Box<dyn Catcher>;


pub struct Routes {
	// maybe store static routes in hashmap??
	raw: Vec<BoxedRawRoute>,
	basic: Vec<BoxedRoute>,
	catcher: Vec<BoxedCatcher>
}

impl Routes {
	pub fn new() -> Self {
		Self{
			raw: vec![],
			basic: vec![],
			catcher: vec![]
		}
	}

	pub fn push_raw<R>(&mut self, route: R)
	where R: RawRoute + 'static {
		self.raw.push(Box::new(route))
	}

	pub fn push<R>(&mut self, route: R)
	where R: Route + 'static {
		self.basic.push(Box::new(route))
	}

	pub fn push_catcher<C>(&mut self, catcher: C)
	where C: Catcher + 'static {
		self.catcher.push(Box::new(catcher))
	}

	pub fn route_raw(
		&self,
		hyper_request: &HyperRequest
	) -> Option<&BoxedRawRoute> {
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
	) -> Option<&BoxedRoute> {
		for route in &self.basic {
			if route.check( request_header ) {
				return Some( route )
			}
		}
		None
	}

	pub fn catchers(&self) -> slice::Iter<'_, BoxedCatcher> {
		self.catcher.iter()
	}
}