use super::{Constraints, BodyAsyncBytesStreamer};

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::body::{Body, Incoming, Frame};

use futures_core::Stream;

use pin_project_lite::pin_project;

use bytes::Bytes;

pin_project! {
	pub struct BodyHttp {
		#[pin]
		inner: BodyAsyncBytesStreamer
	}
}

impl BodyHttp {
	pub(super) fn new(inner: super::Inner, constraints: Constraints) -> Self {
		Self {
			inner: BodyAsyncBytesStreamer::new(inner, constraints)
		}
	}
}

impl Body for BodyHttp {
	type Data = Bytes;
	type Error = io::Error;

	fn poll_frame(
		self: Pin<&mut Self>,
		cx: &mut Context
	) -> Poll<Option<io::Result<Frame<Bytes>>>> {
		let me = self.project();
		match me.inner.poll_next(cx) {
			Poll::Ready(Some(Ok(b))) => Poll::Ready(Some(Ok(Frame::data(b)))),
			Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending
		}
	}
}


pub(super) struct IncomingAsAsyncBytesStream {
	inner: Incoming
}

impl IncomingAsAsyncBytesStream {
	pub fn new(inner: Incoming) -> Self {
		Self { inner }
	}
}

impl Stream for IncomingAsAsyncBytesStream {
	type Item = io::Result<Bytes>;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context
	) -> Poll<Option<io::Result<Bytes>>> {
		let me = self.get_mut();
		// loop to retry to get data
		loop {
			let r = match Pin::new(&mut me.inner).poll_frame(cx) {
				Poll::Ready(Some(Ok(frame))) => {
					match frame.into_data() {
						Some(d) => Poll::Ready(Some(Ok(d))),
						None => continue
					}
				},
				Poll::Ready(Some(Err(e))) => {
					Poll::Ready(Some(Err(io::Error::new(
						io::ErrorKind::Other,
						e
					))))
				},
				Poll::Ready(None) => Poll::Ready(None),
				Poll::Pending => Poll::Pending
			};

			break r
		}
	}
}