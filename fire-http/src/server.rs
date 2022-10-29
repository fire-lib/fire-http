use crate::util::PinnedFuture;
use crate::fire::{self, Wood};

use std::io;
use std::sync::Arc;
use std::net::SocketAddr;
use std::convert::Infallible;

use types::body::BodyHttp;

use tokio::net::TcpListener;

use hyper::Response;
use hyper::service::Service;
use hyper::body::Incoming;
use hyper::server::conn::http1;


pub type HyperRequest = hyper::Request<Incoming>;

// todo replace this function once hyper-util is ready
pub(crate) struct Server {
	listener: TcpListener,
	wood: Arc<Wood>
}

impl Server {
	pub async fn bind(addr: SocketAddr, wood: Wood) -> io::Result<Self> {
		let listener = TcpListener::bind(addr).await?;
		let wood = Arc::new(wood);

		Ok(Self { listener, wood })
	}

	pub fn local_addr(&self) -> io::Result<SocketAddr> {
		self.listener.local_addr()
	}

	pub async fn serve(self) -> io::Result<()> {
		let Server { listener, wood } = self;

		loop {
			let (stream, addr) = listener.accept().await?;
			let service = FireService {
				wood: wood.clone(),
				address: addr
			};
			tokio::task::spawn(async move {
				let r = http1::Builder::new()
					.http1_keep_alive(true)
					.serve_connection(stream, service)
					.with_upgrades().await;
				if let Err(e) = r {
					tracing::error!("Error while service HTTP connection: {e}");
				}
			});
		}
	}
}

pub struct FireService {
	wood: Arc<Wood>,
	address: SocketAddr
}

impl Service<HyperRequest> for FireService {
	type Response = Response<BodyHttp>;
	type Error = Infallible;
	type Future = PinnedFuture<'static, Result<Self::Response, Self::Error>>;

	fn call(&mut self, req: HyperRequest) -> Self::Future {
		PinnedFuture::new(fire::new_spark(self.wood.clone(), req, self.address))
	}
}