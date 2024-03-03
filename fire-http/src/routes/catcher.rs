use crate::header::{RequestHeader, ResponseHeader};
use crate::util::PinnedFuture;
use crate::{Data, Request, Response};

// Catcher Trait
pub trait Catcher: Send + Sync {
	fn check(&self, req: &RequestHeader, res: &ResponseHeader) -> bool;

	// check if every data you expect is in Data
	fn validate_data(&self, _data: &Data) {}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		resp: &'a mut Response,
		data: &'a Data,
	) -> PinnedFuture<'a, crate::Result<()>>;
}
