use crate::util::PinnedFuture;
use crate::{Request, Response, Data};
use crate::header::{RequestHeader, ResponseHeader};

// Catcher Trait
pub trait Catcher: Send +  Sync {
	fn check(&self, req: &RequestHeader, res: &ResponseHeader) -> bool;

	// check if every data you expect is in Data
	fn validate_data(&self, _data: &Data) {}

	fn call<'a>(
		&'a self,
		req: Request,
		res: Response,
		data: &'a Data
	) -> PinnedFuture<'a, crate::Result<Response>>;
}