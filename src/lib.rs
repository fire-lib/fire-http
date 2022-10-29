#![doc = include_str!("../README.md")]

mod data;
pub use data::Data;

pub mod routes;
use routes::{Routes, RawRoute, Route, Catcher};

#[macro_use]
pub mod util;

pub mod into;

pub mod error;
pub use error::{Result, Error};

mod server;

mod fire;
use fire::RequestConfigs;

#[cfg(feature = "fs")]
pub mod fs;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "ws")]
pub mod ws;

use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use std::any::Any;

use tokio::task;
use tokio::net::ToSocketAddrs;

pub use types;
pub use types::{Request, Response, Body, header, body};

/// Prepares a server.
pub async fn build(addr: impl ToSocketAddrs) -> io::Result<FireBuilder> {
	FireBuilder::new(addr).await
}

/// `FireBuilder` gathers all materials needed to light a fire (start a server).
pub struct FireBuilder {
	addr: SocketAddr,
	data: Data,
	routes: Routes,
	configs: RequestConfigs,
	show_startup_msg: bool
}

impl FireBuilder {

	pub(crate) async fn new<A>(addr: A) -> io::Result<Self>
	where A: ToSocketAddrs {
		let addr = tokio::net::lookup_host(addr).await?.next().unwrap();
		Ok(Self {
			addr,
			data: Data::new(),
			routes: Routes::new(),
			configs: RequestConfigs::new(),
			show_startup_msg: true
		})
	}

	/// Returns a reference to the current data.
	pub fn data(&self) -> &Data {
		&self.data
	}

	pub fn add_data<D>(&mut self, data: D)
	where D: Any + Send + Sync {
		self.data.insert(data);
	}

	/// Adds a `RawRoute` to the fire.
	pub fn add_raw_route<R>(&mut self, route: R)
	where R: RawRoute + 'static {
		route.validate_data(&self.data);
		self.routes.push_raw(route)
	}

	/// Adds a `Route` to the fire.
	pub fn add_route<R>(&mut self, route: R)
	where R: Route + 'static {
		route.validate_data(&self.data);
		self.routes.push(route)
	}

	/// Adds a `Catcher` to the fire.
	pub fn add_catcher<C>(&mut self, catcher: C)
	where C: Catcher + 'static {
		catcher.validate_data(&self.data);
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
		self.configs.timeout(timeout)
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

		let wood = fire::Wood::new(self.data, self.routes, self.configs);

		let server = server::Server::bind(self.addr, wood).await
			.map_err(Error::from_server_error)?;

		server.serve().await
			.map_err(Error::from_server_error)
	}

	#[doc(hidden)]
	pub async fn test_light(self) -> (SocketAddr, task::JoinHandle<()>) {
		let wood = fire::Wood::new(self.data, self.routes, self.configs);

		let server = server::Server::bind(self.addr, wood).await.unwrap();

		let addr = server.local_addr().unwrap();
		let task = tokio::spawn(async move {
			server.serve().await.expect("test server failed");
		});

		(addr, task)
	}

}