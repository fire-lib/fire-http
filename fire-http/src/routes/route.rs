use crate::header::RequestHeader;
use crate::util::PinnedFuture;
use crate::{Data, Request, Response};

/// A `Route` is sort of a request handler, routes are checked in order they
/// where added. If a route returns `true` from the `check` method, `call` is
/// executed.
///
/// If possible you should use the provided macros which implement Route for
/// you.
pub trait Route: Send + Sync {
	fn check(&self, req: &RequestHeader) -> bool;

	// check if every data you expect is in Data
	fn validate_data(&self, data: &Data);

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		data: &'a Data,
	) -> PinnedFuture<'a, crate::Result<Response>>;
}

/// A helper function to check if a static url matches a request url. The
/// static uri should never end in a slash except if there is only one slash.
///
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::routes::check_static;
///
/// assert!(check_static("/", "/"));
/// assert!(!check_static("//", "/"));
/// assert!(check_static("/request/uri", "/request/uri"));
/// assert!(check_static("/request/uri/", "/request/uri"));
/// assert!(!check_static("/request/uri//", "/request/uri"));
/// assert!(!check_static("/request/uri/more", "/request/uri"));
/// ```
pub fn check_static(uri_path: &str, s: &str) -> bool {
	uri_path == s
		|| (
			// we don't want to expand /
			s.len() > 1
				&& uri_path.ends_with("/")
				&& &uri_path[..uri_path.len() - 1] == s
		)
}
