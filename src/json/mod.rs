
// serde_json = "1.0"
use serde::Serialize;

pub trait IntoJsonResult<T> {
	fn into_result(self) -> crate::Result<T>;
}

impl<T> IntoJsonResult<T> for crate::Result<T>
where T: Serialize {
	fn into_result(self) -> crate::Result<T> {
		self
	}
}

impl<T> IntoJsonResult<T> for T
where T: Serialize {
	fn into_result(self) -> crate::Result<T> {
		Ok(self)
	}
}

// Basic Route
/// needs to return crate::Result<RetType>
#[macro_export]
macro_rules! json_route {
	($name:ident, $($tt:tt)*) => (
		$crate::json_route!($name<Data>, $($tt)*);
	);
	( $name:ident<$data_ty:ty>, |$check_req:ident| $check_block:block, |$req:ident| -> $ret_type:ty $block:block ) => (
		$crate::json_route!($name<$data_ty>, |$check_req| $check_block, |$req,| -> $ret_type $block);
	);
	( $name:ident<$data_ty:ty>, |$check_req:ident| $check_block:block, |$req:ident, $($data:ident),*| -> $ret_type:ty $block:block ) => (

		pub struct $name;

		impl $crate::routes::Route<$data_ty> for $name {

			fn check(&self, $check_req: &$crate::http::header::RequestHeader) -> bool { $check_block }

			fn call<'a>(
				&'a self,
				$req: &'a mut $crate::request::Request,
				raw_data: &'a $data_ty
			) -> $crate::util::PinnedFuture<'a, $crate::Result<$crate::http::Response>> {

				use $crate::json::IntoJsonResult;

				$crate::util::PinnedFuture::new( async move {

					$( let $data = raw_data.$data(); )*

					let ret: $ret_type = async { $block }.await;
					let res = ret.into_result();

					let to_ser = res?;

					// whos error is that??
					// server error
					let body = $crate::http::Body::serialize(&to_ser)
						.map_err(|e| $crate::Error::new(
							$crate::error::ServerErrorKind::InternalServerError,
							e
						))?;

					// now lets build the response
					Ok(
						$crate::http::Response::builder()
							.content_type($crate::http::header::Mime::Json)
							// content length gets automatically set
							.body(body)
							.build()
					)
				} )
			}

		}

	)
}



// static
#[macro_export]
macro_rules! static_json_route {
	($name:ident, $($tt:tt)*) => (
		$crate::static_json_route!($name<Data>, $($tt)*);
	);
	( $name:ident<$data_ty:ty>, $path:expr, $method:ident, |$($data:ident),*| -> $ret_type:ty $block:block ) => (
		$crate::json_route!( $name<$data_ty>, |req| {
			req.method() == &$crate::http::header::Method::$method &&
			$crate::routes::check_static( req.uri().path(), $path )
		}, |$($data),*| -> $ret_type $block );
	)
}

#[macro_export]
macro_rules! json_get {
	($name:ident, $($tt:tt)*) => (
		$crate::json_get!($name<Data>, $($tt)*);
	);
	($name:ident<$data_ty:ty>, $path:expr, |$($data:ident),*| -> $ret_type:ty $block:block) => (
		$crate::static_json_route!( $name<$data_ty>, $path, Get, |$($data),*| -> $ret_type $block );
	)
}

#[macro_export]
macro_rules! json_post {
	($name:ident, $($tt:tt)*) => (
		$crate::json_post!($name<Data>, $($tt)*);
	);
	( $name:ident<$data_ty:ty>, $path:expr, |$($data:ident),*| -> $ret_type:ty $block:block ) => (
		$crate::static_json_route!( $name, $path, Post, |$($data),*| -> $ret_type $block );
	)
}

#[macro_export]
macro_rules! json_put {
	($name:ident, $($tt:tt)*) => (
		$crate::json_put!($name<Data>, $($tt)*);
	);
	( $name:ident<$data_ty:ty>, $path:expr, |$($data:ident),*| -> $ret_type:ty $block:block ) => (
		$crate::static_json_route!( $name, $path, Put, |$($data),*| -> $ret_type $block );
	)
}

#[macro_export]
macro_rules! json_delete {
	($name:ident, $($tt:tt)*) => (
		$crate::json_delete!($name<Data>, $($tt)*);
	);
	( $name:ident<$data_ty:ty>, $path:expr, |$($data:ident),*| -> $ret_type:ty $block:block ) => (
		$crate::static_json_route!( $name<$data_ty>, $path, Delete, |$($data),*| -> $ret_type $block );
	)
}

#[macro_export]
macro_rules! json_head {
	($name:ident, $($tt:tt)*) => (
		$crate::json_head!($name<Data>, $($tt)*);
	);
	( $name:ident<$data_ty:ty>, $path:expr, |$($data:ident),*| -> $ret_type:ty $block:block ) => (
		$crate::static_json_route!( $name<$data_ty>, $path, Head, |$($data),*| -> $ret_type $block );
	)
}


// dynamic
// TODO reactivate
// #[macro_export]
// macro_rules! dyn_json_route {
// 	( $name:ident, $path:expr, $method:ident, |$req:ident, $fire:ident| -> $ret_type:ty $block:block ) => (
// 		$crate::json_route!( $name, |req| {
// 			req.method() == &$crate::http::header::Method::$method &&
// 			req.uri().path().starts_with( $path )
// 		}, |$req, $fire| -> $ret_type $block );
// 	)
// }

// macro_rules! dyn_json_types {
// 	( $($macro_name:ident -> $macro_type:ident),* ) => ($(
// 		#[macro_export]
// 		macro_rules! $macro_name {
// 			( $name:ident, $path:expr, |$req:ident, $fire:ident| -> $ret_type:ty $block:block ) => (
// 				$crate::dyn_json_route!( $name, $path, $macro_type, |$req, $fire| -> $ret_type $block );
// 			)
// 		}
// 	)*)
// }

// // DEFINITION
// dyn_json_types![
// 	dyn_json_get -> Get,
// 	dyn_json_post -> Post,
// 	dyn_json_put -> Put,
// 	dyn_json_delete -> Delete,
// 	dyn_json_head -> Head
// ];