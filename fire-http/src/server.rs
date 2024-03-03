use crate::fire::{self, Wood};
use crate::util::PinnedFuture;
use crate::{Error, FirePit, Result};

use std::convert::Infallible;
use std::net::SocketAddr;
use std::result::Result as StdResult;
use std::sync::Arc;

use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use types::body::BodyHttp;

use hyper::body::Incoming;
use hyper::service::Service;
use hyper::Response;

pub type HyperRequest = hyper::Request<Incoming>;

use tokio::net::TcpListener;

// todo replace this function once hyper-util is ready
pub(crate) struct Server {
	listener: TcpListener,
	wood: Arc<Wood>,
}

impl Server {
	pub(crate) async fn bind(
		addr: SocketAddr,
		wood: Arc<Wood>,
	) -> Result<Self> {
		let listener = TcpListener::bind(&addr)
			.await
			.map_err(Error::from_server_error)?;

		Ok(Self { listener, wood })
	}

	pub fn local_addr(&self) -> Result<SocketAddr> {
		self.listener.local_addr().map_err(Error::from_server_error)
	}

	pub async fn serve(self) -> Result<()> {
		loop {
			let (stream, address) = self
				.listener
				.accept()
				.await
				.map_err(Error::from_server_error)?;

			let io = TokioIo::new(stream);

			let service = FireService {
				wood: self.wood.clone(),
				address,
			};

			tokio::task::spawn(async move {
				if let Err(err) = Builder::new(TokioExecutor::new())
					.serve_connection_with_upgrades(io, service)
					.await
				{
					tracing::error!("Error serving connection: {:?}", err);
				}
			});
		}
	}
}

pub struct FireService {
	wood: Arc<Wood>,
	address: SocketAddr,
}

impl FireService {
	/// Creates a new FireService which can be passed to a hyper server.
	pub fn new(pit: FirePit, address: SocketAddr) -> Self {
		Self {
			wood: pit.wood,
			address,
		}
	}
}

impl Service<HyperRequest> for FireService {
	type Response = Response<BodyHttp>;
	type Error = Infallible;
	type Future = PinnedFuture<'static, StdResult<Self::Response, Self::Error>>;

	fn call(&self, req: HyperRequest) -> Self::Future {
		let wood = self.wood.clone();
		let address = self.address;
		PinnedFuture::new(async move {
			fire::route_hyper(&wood, req, address).await
		})
	}
}
