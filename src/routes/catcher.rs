
use crate::util::PinnedFuture;
use crate::request::Request;

use http::header::{ RequestHeader, ResponseHeader };
use http::response::Response;

// Catcher Trait
pub trait Catcher<D> : Send +  Sync {
	fn check(&self, req: &RequestHeader, res: &ResponseHeader) -> bool;
	fn call<'a>(
		&'a self,
		req: Request,
		res: Response,
		data: &'a D
	) -> PinnedFuture<'a, crate::Result<Response>>;
}


#[macro_export]
macro_rules! catcher {
	($name:ident, $($tt:tt)*) => (
		$crate::catcher!($name<Data>, $($tt)*);
	);
	(
		$name:ident<$data_ty:ty>,
		|$check_req:ident, $check_res:ident| $check_block:block,
		|$req:ident, $res:ident| -> $ret_type:ty $block:block
	) => (
		$crate::catcher!(
			$name<$data_ty>,
			|$check_req, $check_res| $check_block,
			|$req, $res,| -> $ret_type $block
		);
	);
	(
		$name:ident<$data_ty:ty>,
		|$check_req:ident, $check_res:ident| $check_block:block,
		|$req:ident, $res:ident, $( $data:ident ),*| -> $ret_type:ty $block:block
	) => (

		pub struct $name;

		impl $crate::routes::Catcher<$data_ty> for $name {

			fn check(
				&self,
				$check_req: &$crate::http::header::RequestHeader,
				$check_res: &$crate::http::header::ResponseHeader
			) -> bool { $check_block }

			fn call<'a>(
				&'a self,
				mut $req: $crate::request::Request,
				mut $res: $crate::http::response::Response,
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