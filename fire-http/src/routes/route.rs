use crate::util::PinnedFuture;
use crate::{Request, Response, Data};
use crate::header::RequestHeader;

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
		data: &'a Data
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
pub fn check_static(uri_path: &str, s: &'static str) -> bool {
	uri_path == s ||
	(
		// we don't want to expand /
		s.len() > 1 &&
		uri_path.ends_with("/") &&
		&uri_path[..uri_path.len() - 1] == s
	)
}


// ////// MACROS


/// Basic Route
///
/// If possible use the other macros.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::route;
/// use fire::routes::check_static;
/// 
/// route! {
/// 	RouteName,
/// 	|req_header| {
/// 		check_static(req_header.uri().path(), "/")
/// 	},
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! route {
	(
		$name:ident,
		|$check_req:ident| $check_block:block,
		|$req:ident| -> $ret_ty:ty $block:block
	) => (
		$crate::route!(
			$name,
			|$check_req| $check_block,
			|$req,| -> $ret_ty $block
		);
	);
	(
		$name:ident,
		|$check_req:ident| $check_block:block,
		|$req:ident, $( $data:ident: $data_ty:ty ),*| -> $ret_ty:ty
		$block:block
	) => (

		pub struct $name;

		impl $crate::routes::Route for $name {

			fn check(
				&self,
				$check_req: &$crate::header::RequestHeader
			) -> bool {
				$check_block
			}

			fn validate_data(&self, data: &$crate::Data) {
				$(
					assert!(data.exists::<$data_ty>());
				)*
			}

			fn call<'a>(
				&'a self,
				req: &'a mut $crate::Request,
				raw_data: &'a $crate::Data
			) -> $crate::util::PinnedFuture<'a, $crate::Result<$crate::Response>> {

				use $crate::into::IntoRouteResult;

				async fn route(
					$req: &mut $crate::Request,
					$( $data: &$data_ty ),*
				) -> $ret_ty {
					$block
				}

				$crate::util::PinnedFuture::new(async move {
					route(
						req,
						$(
							raw_data.get::<$data_ty>().unwrap()
						),*
					).await.into_route_result()
				})
			}

		}

	)
}


/// Static Route
///
/// If possible use the other macros.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::static_route;
/// 
/// static_route! {
/// 	RouteName, "/", GET,
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! static_route {
	(
		$name:ident,
		$path:expr,
		$method:ident,
		|$req:ident| $($tt:tt)*
	) => (
		$crate::static_route!(
			$name, $path, $method, |$req,| $($tt)*
		);
	);
	(
		$name:ident,
		$path:expr,
		$method:ident,
		|$req:ident, $($data:ident: $data_ty:ty),*| -> $ret_type:ty $block:block
	) => (
		$crate::route!(
			$name,
			|req| {
				req.method() == &$crate::header::Method::$method &&
				$crate::routes::check_static(req.uri().path(), $path)
			},
			|$req, $($data: $data_ty),*| -> $ret_type $block
		);
	)
}

/// Static get handler.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::get;
/// 
/// get! {
/// 	RouteName, "/",
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! get {
	(
		$name:ident,
		$path:expr,
		| $($tt:tt)*
	) => (
		$crate::static_route!(
			$name,
			$path,
			GET,
			| $($tt)*
		);
	)
}

/// Static post handler.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::post;
/// 
/// post! {
/// 	RouteName, "/",
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! post {
	(
		$name:ident,
		$path:expr,
		| $($tt:tt)*
	) => (
		$crate::static_route!(
			$name,
			$path,
			POST,
			| $($tt)*
		);
	)
}

/// Static put handler.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::put;
/// 
/// put! {
/// 	RouteName, "/",
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! put {
	(
		$name:ident,
		$path:expr,
		| $($tt:tt)*
	) => (
		$crate::static_route!(
			$name,
			$path,
			PUT,
			| $($tt)*
		);
	)
}

/// Static delete handler.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::delete;
/// 
/// delete! {
/// 	RouteName, "/",
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! delete {
	(
		$name:ident,
		$path:expr,
		| $($tt:tt)*
	) => (
		$crate::static_route!(
			$name,
			$path,
			DELETE,
			| $($tt)*
		);
	)
}

/// Static head handler.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::head;
/// 
/// head! {
/// 	RouteName, "/",
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! head {
	(
		$name:ident,
		$path:expr,
		| $($tt:tt)*
	) => (
		$crate::static_route!(
			$name,
			$path,
			HEAD,
			| $($tt)*
		);
	)
}


/// Dynamic Route
/// 
/// A dynamic route matches if the start of the path matches the request uri.
///
/// If possible use the other macros.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::dyn_route;
/// 
/// dyn_route! {
/// 	RouteName, "/files/", GET,
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! dyn_route {
	(
		$name:ident,
		$path:expr,
		$method:ident,
		| $($tt:tt)*
	) => (
		$crate::route!(
			$name,
			|req| {
				req.method() == &$crate::header::Method::$method &&
				req.uri().path().starts_with($path)
			},
			| $($tt)*
		);
	)
}

/// Dynamic get request handler.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::dyn_get;
/// 
/// dyn_get! {
/// 	RouteName, "/files/",
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! dyn_get {
	(
		$name:ident,
		$path:expr,
		| $($tt:tt)*
	) => (
		$crate::dyn_route!(
			$name,
			$path,
			GET,
			| $($tt)*
		);
	)
}

/// Dynamic post request handler.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::dyn_post;
/// 
/// dyn_post! {
/// 	RouteName, "/files/",
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! dyn_post {
	(
		$name:ident,
		$path:expr,
		| $($tt:tt)*
	) => (
		$crate::dyn_route!(
			$name,
			$path,
			POST,
			| $($tt)*
		);
	)
}

/// Dynamic put request handler.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::dyn_put;
/// 
/// dyn_put! {
/// 	RouteName, "/files/",
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! dyn_put {
	(
		$name:ident,
		$path:expr,
		| $($tt:tt)*
	) => (
		$crate::dyn_route!(
			$name,
			$path,
			PUT,
			| $($tt)*
		);
	)
}

/// Dynamic delete request handler.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::dyn_delete;
/// 
/// dyn_delete! {
/// 	RouteName, "/files/",
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! dyn_delete {
	(
		$name:ident,
		$path:expr,
		| $($tt:tt)*
	) => (
		$crate::dyn_route!(
			$name,
			$path,
			DELETE,
			| $($tt)*
		);
	)
}

/// Dynamic head request handler.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::dyn_head;
/// 
/// dyn_head! {
/// 	RouteName, "/files/",
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! dyn_head {
	(
		$name:ident,
		$path:expr,
		| $($tt:tt)*
	) => (
		$crate::dyn_route!(
			$name,
			$path,
			HEAD,
			| $($tt)*
		);
	)
}
