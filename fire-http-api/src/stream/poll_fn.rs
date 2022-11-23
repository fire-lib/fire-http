/// use the std::future::poll_fn function after the minimal rust version changes
/// to 1.64

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) fn poll_fn<T, F>(f: F) -> PollFn<F>
where F: FnMut(&mut Context<'_>) -> Poll<T> {
	PollFn { f }
}

pub(crate) struct PollFn<F> {
	f: F
}

impl<F: Unpin> Unpin for PollFn<F> {}

impl<F> fmt::Debug for PollFn<F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("PollFn").finish()
	}
}

impl<T, F> Future for PollFn<F>
where F: FnMut(&mut Context<'_>) -> Poll<T> {
	type Output = T;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
		// SAFETY: We are not moving out of the pinned field.
		(unsafe { &mut self.get_unchecked_mut().f })(cx)
	}
}