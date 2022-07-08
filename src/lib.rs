#![doc = include_str!("../README.md")]

pub mod routes;
use routes::{Routes, RawRoute, Route, Catcher};

pub mod parse;
use parse::socket::{ParseResult, ParseSocketAddr};

#[macro_use]
pub mod util;

pub mod into;

pub mod error;
pub use error::{Result, Error};

pub mod request;

mod fire;
use fire::RequestConfigs;

mod service;
use service::MakeFireService;

#[cfg(feature = "fs")]
pub mod fs;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "ws")]
pub mod ws;

use std::net::SocketAddr;
use std::future::Future;
use std::time::Duration;

pub use http;
pub use http::header;
pub use http::body;
pub use http::response;

/// Prepares a server.
pub fn build<A, D>(addr: A, data: D) -> ParseResult<FireBuilder<D>>
where
	A: ParseSocketAddr,
	D: Data {
	FireBuilder::new(addr, data.into())
}

/// `FireBuilder` gathers all materials needed to light a fire (start a server).
pub struct FireBuilder<D> {
	addr: SocketAddr,
	data: D,
	routes: Routes<D>,
	configs: RequestConfigs,
	show_startup_msg: bool
}

impl<D: Data> FireBuilder<D> {

	pub(crate) fn new<A>(addr: A, data: D) -> ParseResult<Self>
	where A: ParseSocketAddr {
		Ok(Self {
			addr: addr.parse()?,
			data,
			routes: Routes::new(),
			configs: RequestConfigs::new(),
			show_startup_msg: true
		})
	}

	/// Returns a reference to the current data.
	pub fn data(&self) -> &D {
		&self.data
	}

	/// Adds a `RawRoute` to the fire.
	pub fn add_raw_route<R>(&mut self, route: R)
	where R: RawRoute<D> + 'static {
		self.routes.push_raw(route)
	}

	/// Adds a `Route` to the fire.
	pub fn add_route<R>(&mut self, route: R)
	where R: Route<D> + 'static {
		self.routes.push(route)
	}

	/// Adds a `Catcher` to the fire.
	pub fn add_catcher<C>(&mut self, catcher: C)
	where C: Catcher<D> + 'static {
		self.routes.push_catcher(catcher)
	}

	/// Sets the request size limit. The default is 4 kilobytes.
	/// 
	/// This can be changed in every Route.
	/// 
	/// ## Panics
	/// If the size is zero.
	pub fn request_size_limit(&mut self, size_limit: usize) {
		self.configs.size_limit(size_limit)
	}

	/// Sets the request timeout. The default is 60 seconds.
	/// 
	/// This can be changed in every Route.
	pub fn request_timeout(&mut self, timeout: Duration) {
		self.configs.timeout(timeout);
	}

	/// Prevents the fire from showing a message when the server get's started.
	pub fn hide_startup_message(&mut self) {
		self.show_startup_msg = false;
	}

	/// Lights the fire, which starts the server.
	/// 
	/// ## Note
	/// Under normal conditions this function should run forever.
	pub async fn light(self) -> Result<()> {

		if self.show_startup_msg {
			println!("Running server on addr: {}", self.addr);
		}

		let fire = fire::Wood::new(self.data, self.routes, self.configs);

		let make_service = MakeFireService::new(fire);

		let builder = hyper::Server::bind(&self.addr);
		// builder https://docs.rs/hyper/0.13.6/hyper/server/struct.Builder.html
		builder.serve(make_service).await
			.map_err(Error::from_server_error)
	}

	#[doc(hidden)]
	pub fn test_light(self) -> (std::net::SocketAddr, impl Future) {

		let fire = fire::Wood::new(self.data, self.routes, self.configs);

		let make_service = MakeFireService::new(fire);

		let builder = hyper::Server::bind(&self.addr);
		// builder https://docs.rs/hyper/0.13.6/hyper/server/struct.Builder.html
		let fut = builder.serve(make_service);

		(fut.local_addr(), fut)
	}

}

/// To access data from request handlers you need to create a data struct
/// every request receivers a reference to this struct.
/// 
/// To access a value a method with the same name needs to be defined. This
/// macro simplifies the creation of such methods.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::data_struct;
/// 
/// data_struct! {
/// 	#[derive(Debug)]
/// 	pub struct Data {
/// 		/// Comment here
/// 		field_1: String,
/// 		another_field: usize
/// 	}
/// }
/// 
/// let data = Data {
/// 	field_1: "Field1".into(),
/// 	another_field: 10
/// };
/// // automatically generates getter functions
/// let field_1 = data.field_1();
/// let another_field = data.another_field();
/// assert_eq!(field_1, "Field1");
/// assert_eq!(another_field, &10);
/// ```
#[macro_export]
macro_rules! data_struct {
	(
		IMPL,
		$(#[$attr:meta])*
		($($vis:tt)*) $name:ident {
			$(
				$(#[$field_attr:meta])*
				$field:ident: $field_type:ty
			),*
		}
	) => (
		$(#[$attr])*
		$($vis)* struct $name {
			$(
				$(#[$field_attr])*
				$field: $field_type
			),*
		}

		impl $name {

			pub fn data(&self) -> &Self { self }

			$(
				pub fn $field(&self) -> &$field_type {
					&self.$field
				}
			)*
		}
	);
	($(#[$attr:meta])* pub struct $($toks:tt)*) => (
		data_struct!( IMPL, $(#[$attr])* (pub) $($toks)* );
	);
	($(#[$attr:meta])* pub ($($vis:tt)+) struct $($toks:tt)*) => (
		data_struct!( IMPL, $(#[$attr])* (pub ($($vis)+)) $($toks)* );
	);
	($(#[$attr:meta])* struct $($toks:tt)*) => (
		data_struct!( IMPL, $(#[$attr])* () $($toks)* );
	)
}

/// A trait that simplifies the bounds on other methods or structs.
/// 
/// You don't need to implement this since it's implemented for every compatible
/// type.
pub trait Data: Send + Sync + 'static {}

impl<T> Data for T
where T: Send + Sync + 'static {}