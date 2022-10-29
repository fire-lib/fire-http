use crate::util::PinnedFuture;
use crate::{Response, Data};

pub use crate::server::HyperRequest;

/// A `RawRoute` is the more powerfull brother/sister to `Route`. It get's
/// executed before `Route`.
/// The `RawRoute` should only be needed if you implement something lower level
/// like websockets and need access to the underlying hyper types.
pub trait RawRoute: Send + Sync {
	fn check(&self, req: &HyperRequest) -> bool;

	// check if every data you expect is in Data
	fn validate_data(&self, data: &Data);

	fn call<'a>(
		&'a self,
		req: &'a mut HyperRequest,
		data: &'a Data
	) -> PinnedFuture<'a, Option<crate::Result<Response>>>;
}