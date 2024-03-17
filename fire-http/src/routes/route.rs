use super::{ParamsNames, PathParams};

use crate::header::Method;
use crate::util::PinnedFuture;
use crate::{Data, Request, Response};

use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct RoutePath {
	pub method: Option<Method>,
	pub path: Cow<'static, str>,
}

/// A `Route` is sort of a request handler
///
/// If possible you should use the provided macros which implement Route for
/// you.
pub trait Route: Send + Sync {
	// check if every data you expect is in Data
	fn validate_data(&self, params: &ParamsNames, data: &Data);

	// get's only called once
	fn path(&self) -> RoutePath;

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		params: &'a PathParams,
		data: &'a Data,
	) -> PinnedFuture<'a, crate::Result<Response>>;
}
