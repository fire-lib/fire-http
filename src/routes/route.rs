
use crate::util::PinnedFuture;
use crate::request::Request;

use http::response::Response;
use http::header::RequestHeader;

/// A `Route` is sort of a request handler, routes are checked in order they
/// where added. If a route returns `true` from the `check` method, `call` is
/// executed.
/// 
/// If possible you should use the provided macros which implement Route for
/// you.
pub trait Route<D>: Send + Sync {
	fn check(&self, req: &RequestHeader) -> bool;
	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		data: &'a D
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
/// type Data = ();
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
	($name:ident, $($tt:tt)* ) => (
		$crate::route!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		|$check_req:ident| $check_block:block,
		|$req:ident| -> $ret_type:ty $block:block
	) => (
		$crate::route!(
			$name<$data_ty>,
			|$check_req| $check_block,
			|$req,| -> $ret_type $block
		);
	);
	(
		$name:ident<$data_ty:ty>,
		|$check_req:ident| $check_block:block,
		|$req:ident, $( $data:ident ),*| -> $ret_type:ty
		$block:block
	) => (

		pub struct $name;

		impl $crate::routes::Route<$data_ty> for $name {

			fn check(
				&self,
				$check_req: &$crate::http::header::RequestHeader
			) -> bool {
				$check_block
			}

			fn call<'a>(
				&'a self,
				$req: &'a mut $crate::request::Request,
				raw_data: &'a $data_ty
			) -> $crate::util::PinnedFuture<'a, $crate::Result<$crate::http::Response>> {

				use $crate::into::IntoRouteResult;

				$crate::util::PinnedFuture::new( async move {

					$( let $data = raw_data.$data(); )*

					let data: $ret_type = async { $block }.await;
					data.into_route_result()
				} )
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
/// type Data = ();
/// 
/// static_route! {
/// 	RouteName, "/", Get,
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! static_route {
	($name:ident, $($tt:tt)*) => (
		$crate::static_route!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		$method:ident,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::route!(
			$name<$data_ty>,
			|req| {
				req.method() == &$crate::http::header::Method::$method &&
				$crate::routes::check_static(req.uri().path(), $path)
			},
			|$($data),*| -> $ret_type $block
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
/// type Data = ();
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
	($name:ident, $($tt:tt)*) => (
		$crate::get!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::static_route!(
			$name<$data_ty>,
			$path,
			Get,
			|$($data),*| -> $ret_type $block
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
/// type Data = ();
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
	($name:ident, $($tt:tt)*) => (
		$crate::post!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::static_route!(
			$name<$data_ty>,
			$path,
			Post,
			|$($data),*| -> $ret_type $block
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
/// type Data = ();
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
	($name:ident, $($tt:tt)*) => (
		$crate::put!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::static_route!(
			$name<$data_ty>,
			$path,
			Put,
			|$($data),*| -> $ret_type $block
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
/// type Data = ();
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
	($name:ident, $($tt:tt)*) => (
		$crate::delete!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::static_route!(
			$name<$data_ty>,
			$path,
			Delete,
			|$($data),*| -> $ret_type $block
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
/// type Data = ();
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
	($name:ident, $($tt:tt)*) => (
		$crate::head!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::static_route!(
			$name<$data_ty>,
			$path,
			Head,
			|$($data),*| -> $ret_type $block
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
/// type Data = ();
/// 
/// dyn_route! {
/// 	RouteName, "/files/", Get,
/// 	|_req| -> &'static str {
/// 		"Hello, World!"
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! dyn_route {
	($name:ident, $($tt:tt)*) => (
		$crate::dyn_route!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		$method:ident,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::route!(
			$name<$data_ty>,
			|req| {
				req.method() == &$crate::http::header::Method::$method &&
				req.uri().path().starts_with($path)
			},
			|$($data),*| -> $ret_type
			$block
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
/// type Data = ();
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
	($name:ident, $($tt:tt)*) => (
		$crate::dyn_get!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::dyn_route!(
			$name<$data_ty>,
			$path,
			Get,
			|$($data),*| -> $ret_type $block
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
/// type Data = ();
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
	($name:ident, $($tt:tt)*) => (
		$crate::dyn_post!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::dyn_route!(
			$name<$data_ty>,
			$path,
			Post,
			|$($data),*| -> $ret_type $block
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
/// type Data = ();
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
	($name:ident, $($tt:tt)*) => (
		$crate::dyn_put!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::dyn_route!(
			$name<$data_ty>,
			$path,
			Put,
			|$($data),*| -> $ret_type $block
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
/// type Data = ();
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
	($name:ident, $($tt:tt)*) => (
		$crate::dyn_delete!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::dyn_route!(
			$name<$data_ty>,
			$path,
			Delete,
			|$($data),*| -> $ret_type $block
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
/// type Data = ();
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
	($name:ident, $($tt:tt)*) => (
		$crate::dyn_head!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		$path:expr,
		|$($data:ident),*| -> $ret_type:ty $block:block
	) => (
		$crate::dyn_route!(
			$name<$data_ty>,
			$path,
			Head,
			|$($data),*| -> $ret_type $block
		);
	)
}
