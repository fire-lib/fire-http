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
	(
		$name:ident,
		|$check_req:ident| $check_block:block,
		|$req:ident| -> $ret_ty:ty $block:block
	) => (
		$crate::json_route!(
			$name,
			|$check_req| $check_block,
			|$req,| -> $ret_ty $block
		);
	);
	(
		$name:ident,
		|$check_req:ident| $check_block:block,
		|$req:ident, $($data:ident: $data_ty:ty),*| -> $ret_ty:ty $block:block
	) => (

		pub struct $name;

		impl $crate::routes::Route for $name {

			fn check(
				&self,
				$check_req: &$crate::header::RequestHeader
			) -> bool { $check_block }

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

				use $crate::json::IntoJsonResult;

				async fn route(
					$req: &mut $crate::Request,
					$( $data: &$data_ty ),*
				) -> $ret_ty {
					$block
				}

				$crate::util::PinnedFuture::new(async move {

					let serializeable = route(
						req,
						$(
							raw_data.get::<$data_ty>().unwrap()
						),*
					).await.into_result()?;

					// whos error is that??
					// server error
					let body = $crate::Body::serialize(&serializeable)
						.map_err(|e| $crate::Error::new(
							$crate::error::ServerErrorKind::InternalServerError,
							e
						))?;

					// now lets build the response
					let resp = $crate::Response::builder()
						.content_type($crate::header::Mime::JSON)
						// content length gets automatically set
						.body(body)
						.build();
					
					Ok(resp)
				})
			}

		}

	)
}



// static
#[macro_export]
macro_rules! static_json_route {
	(
		$name:ident, $path:expr, $method:ident,
		| $($tt:tt)*
	) => (
		$crate::json_route!( $name,
			|req| {
				req.method() == &$crate::header::Method::$method &&
				$crate::routes::check_static(req.uri().path(), $path)
			},
			| $($tt)*
		);
	)
}

#[macro_export]
macro_rules! json_get {
	(
		$name:ident, $path:expr,
		| $($tt:tt)*
	) => (
		$crate::static_json_route!($name, $path, GET, | $($tt)*);
	)
}

#[macro_export]
macro_rules! json_post {
	(
		$name:ident, $path:expr,
		| $($tt:tt)*
	) => (
		$crate::static_json_route!($name, $path, POST, | $($tt)*);
	)
}

#[macro_export]
macro_rules! json_put {
	(
		$name:ident, $path:expr,
		| $($tt:tt)*
	) => (
		$crate::static_json_route!($name, $path, PUT, | $($tt)*);
	)
}

#[macro_export]
macro_rules! json_delete {
	(
		$name:ident, $path:expr,
		| $($tt:tt)*
	) => (
		$crate::static_json_route!($name, $path, DELETE, | $($tt)*);
	)
}

#[macro_export]
macro_rules! json_head {
	(
		$name:ident, $path:expr,
		| $($tt:tt)*
	) => (
		$crate::static_json_route!($name, $path, HEAD, | $($tt)*);
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