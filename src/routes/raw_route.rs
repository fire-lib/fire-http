
use crate::request::{HyperRequest, RequestBuilder};
use crate::util::PinnedFuture;

use http::Response;

/// A `RawRoute` is the more powerfull brother/sister to `Route`. It get's
/// executed before `Route`.
/// The `RawRoute` should only be needed if you implement something lower level
/// like websockets and need access to the underlying hyper types.
pub trait RawRoute<D>: Send + Sync {
	fn check(&self, req: &HyperRequest) -> bool;
	fn call<'a>(
		&'a self,
		req: &'a mut RequestBuilder<'_>,
		data: &'a D
	) -> PinnedFuture<'a, Option<crate::Result<Response>>>;
}