#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

mod data;
pub use data::Data;

pub mod routes;
use routes::{Routes, RawRoute, Route, Catcher};

#[macro_use]
pub mod util;

pub mod into;
use into::IntoRoute;

pub mod error;
pub use error::{Result, Error};

mod server;
use server::Server;

mod fire;
use fire::{RequestConfigs, Wood};

#[cfg(feature = "fs")]
#[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
pub mod fs;

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub mod json;

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
pub mod ws;

use std::net::SocketAddr;
use std::time::Duration;
use std::any::Any;
use std::sync::Arc;

use tokio::net::ToSocketAddrs;

pub use types;
pub use types::{Request, Response, Body, header, body};

pub use codegen::*;

/// Prepares a server.
pub async fn build(addr: impl ToSocketAddrs) -> Result<FireBuilder> {
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
	pub(crate) async fn new<A>(addr: A) -> Result<Self>
	where A: ToSocketAddrs {
		let addr = tokio::net::lookup_host(addr).await
			.map_err(Error::from_server_error)?
			.next().unwrap();
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
	where R: IntoRoute + 'static {
		let route = route.into_route();
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

	/// Binds to the address and prepares to serve requests.
	/// 
	/// You need to call ignite on the `Fire` so that it starts handling
	/// requests.
	pub async fn build(self) -> Result<Fire> {
		let wood = Arc::new(Wood::new(self.data, self.routes, self.configs));

		let server = Server::bind(self.addr, wood.clone()).await?;

		Ok(Fire {
			wood, server,
			show_startup_msg: self.show_startup_msg
		})
	}

	/// Ignites the fire, which starts the server.
	/// 
	/// ## Note
	/// Under normal conditions this function should run forever.
	pub async fn ignite(self) -> Result<()> {
		let fire = self.build().await?;
		fire.ignite().await
	}
}

/// A Fire that is ready to be ignited.
pub struct Fire {
	wood: Arc<Wood>,
	server: Server,
	show_startup_msg: bool
}

impl Fire {
	pub fn local_addr(&self) -> Option<SocketAddr> {
		self.server.local_addr().ok()
	}

	pub fn pit(&self) -> FirePit {
		FirePit { wood: self.wood.clone() }
	}

	pub async fn ignite(self) -> Result<()> {
		if self.show_startup_msg {
			eprintln!("Running server on addr: {}", self.local_addr().unwrap());
		}

		self.server.serve().await
	}
}

#[derive(Clone)]
pub struct FirePit {
	wood: Arc<Wood>
}

impl FirePit {
	pub fn data(&self) -> &Data {
		self.wood.data()
	}

	/// Routes the request to normal routes and returns their result.
	/// 
	/// Useful for tests and niche applications.
	/// 
	/// Returns None if no route was found matching the request.
	pub async fn route(&self, req: &mut Request) -> Option<Result<Response>> {
		fire::route(&self.wood, req).await
	}
}