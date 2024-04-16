#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

mod resources;
pub use resources::Resources;

pub mod state;

pub mod routes;
use routes::{Catcher, ParamsNames, RawRoute, Route, Routes};

#[macro_use]
pub mod util;

pub mod into;
use into::IntoRoute;

pub mod error;
pub use error::{Error, Result};

pub mod extractor;

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

#[cfg(feature = "graphql")]
#[cfg_attr(docsrs, doc(cfg(feature = "graphql")))]
pub mod graphql;

pub mod service {
	pub use crate::server::FireService;
}

use std::any::Any;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::ToSocketAddrs;
use tokio::task::JoinHandle;

pub use types;
pub use types::{body, header, Body, Request, Response};

pub use codegen::*;

/// Prepares a server.
pub async fn build(addr: impl ToSocketAddrs) -> Result<FireBuilder> {
	FireBuilder::new(addr).await
}

/// `FireBuilder` gathers all materials needed to light a fire (start a server).
pub struct FireBuilder {
	addr: SocketAddr,
	data: Resources,
	routes: Routes,
	configs: RequestConfigs,
	show_startup_msg: bool,
}

impl FireBuilder {
	pub(crate) async fn new<A>(addr: A) -> Result<Self>
	where
		A: ToSocketAddrs,
	{
		let addr = tokio::net::lookup_host(addr)
			.await
			.map_err(Error::from_server_error)?
			.next()
			.unwrap();
		Ok(Self {
			addr,
			data: Resources::new(),
			routes: Routes::new(),
			configs: RequestConfigs::new(),
			show_startup_msg: true,
		})
	}

	/// Returns a reference to the current data.
	pub fn data(&self) -> &Resources {
		&self.data
	}

	pub fn add_data<D>(&mut self, data: D)
	where
		D: Any + Send + Sync,
	{
		self.data.insert(data);
	}

	/// Adds a `RawRoute` to the fire.
	pub fn add_raw_route<R>(&mut self, route: R)
	where
		R: RawRoute + 'static,
	{
		let path = route.path();
		let names = ParamsNames::parse(&path.path);
		route.validate_data(&names, &self.data);
		self.routes.push_raw(path, route)
	}

	/// Adds a `Route` to the fire.
	pub fn add_route<R>(&mut self, route: R)
	where
		R: IntoRoute + 'static,
	{
		let route = route.into_route();
		let path = route.path();
		let names = ParamsNames::parse(&path.path);
		route.validate_requirements(&names, &self.data);
		self.routes.push(path, route)
	}

	/// Adds a `Catcher` to the fire.
	pub fn add_catcher<C>(&mut self, catcher: C)
	where
		C: Catcher + 'static,
	{
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
			wood,
			server,
			show_startup_msg: self.show_startup_msg,
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

	/// Ignites the fire, and spawns it on a new tokio task.
	///
	/// ## Note
	/// Under normal conditions this task should run forever.
	pub fn ignite_task(self) -> JoinHandle<()> {
		tokio::spawn(async move { self.ignite().await.unwrap() })
	}

	/// Creates a FirePit without starting the server.
	///
	/// In most cases you should use `build` and then call `pit` on the `Fire`.
	///
	/// Creating a `FirePit` might be useful for testing or if you want to
	/// manually create a server.
	pub fn into_pit(self) -> FirePit {
		let wood = Arc::new(Wood::new(self.data, self.routes, self.configs));

		FirePit { wood }
	}
}

/// A Fire that is ready to be ignited.
pub struct Fire {
	wood: Arc<Wood>,
	server: Server,
	show_startup_msg: bool,
}

impl Fire {
	pub fn local_addr(&self) -> Option<SocketAddr> {
		self.server.local_addr().ok()
	}

	pub fn pit(&self) -> FirePit {
		FirePit {
			wood: self.wood.clone(),
		}
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
	wood: Arc<Wood>,
}

impl FirePit {
	pub fn data(&self) -> &Resources {
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
