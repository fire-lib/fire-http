
use crate::Data;
use crate::util::PinnedFuture;
use crate::fire::{ self, Wood, MoreWood };
use crate::request::HyperRequest;

use std::sync::Arc;
use std::task::Poll;
use std::pin::Pin;
use std::future::Future;
use std::task::Context;
use std::net::SocketAddr;
use std::convert::Infallible;

use http::body::FireHttpBody;

use hyper::Response;
use hyper::service::Service;
use hyper::server::conn::AddrStream;


pub struct FireService<D> {
	wood: MoreWood<D>,
	address: SocketAddr
}

// https://docs.rs/hyper/0.14.2/src/hyper/service/http.rs.html#8-27
impl<D: Data> Service<HyperRequest> for FireService<D> {

	type Response = Response<FireHttpBody>;
	type Error = Infallible;// should probably not return this error
	type Future = PinnedFuture<'static, Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: HyperRequest) -> Self::Future {
		PinnedFuture::new(fire::new_spark(self.wood.clone(), req, self.address))
	}

}


pub struct MakeFireService<D> {
	wood: MoreWood<D>
}

impl<D> MakeFireService<D> {
	pub fn new( wood: Wood<D> ) -> Self {
		Self { wood: Arc::new(wood) }
	}
}

// https://docs.rs/hyper/0.14.2/src/hyper/service/make.rs.html#138-144
impl<'t, D> Service<&'t AddrStream> for MakeFireService<D> {

	type Response = FireService<D>;
	type Error = Infallible;
	type Future = FutureService<D>;// Future<Output = Result<Self::Response, Self::Error>>

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, addr_stream: &'t AddrStream) -> Self::Future {
		let wood = self.wood.clone();
		let address = addr_stream.remote_addr();
		FutureService::new( FireService { wood, address } )
	}

}

pub struct FutureService<D>(Option<FireService<D>>);

impl<D> FutureService<D> {

	fn new(res: FireService<D>) -> Self {
		Self(Some(res))
	}

	fn take(&mut self) -> Option<FireService<D>> {
		self.0.take()
	}

}

impl<D> Future for FutureService<D> {

	type Output = Result<FireService<D>, Infallible>;

	/// ## Panics
	/// If called more than once
	fn poll(mut self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
		Poll::Ready(Ok(
			self.take().expect("this future should only be called once")
		))
	}
}