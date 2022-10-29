use crate::util::PinnedFuture;
use crate::{Request, Response, Data};
use crate::header::{RequestHeader, ResponseHeader};


// Catcher Trait
pub trait Catcher: Send +  Sync {
	fn check(&self, req: &RequestHeader, res: &ResponseHeader) -> bool;

	// check if every data you expect is in Data
	fn validate_data(&self, data: &Data);

	fn call<'a>(
		&'a self,
		req: Request,
		res: Response,
		data: &'a Data
	) -> PinnedFuture<'a, crate::Result<Response>>;
}


#[macro_export]
macro_rules! catcher {
	(
		$name:ident,
		|$check_req:ident, $check_res:ident| $check_block:block,
		|$req:ident, $res:ident| -> $ret_type:ty $block:block
	) => (
		$crate::catcher!(
			$name,
			|$check_req, $check_res| $check_block,
			|$req, $res,| -> $ret_type $block
		);
	);
	(
		$name:ident,
		|$check_req:ident, $check_res:ident| $check_block:block,
		|$req:ident, $res:ident, $( $data:ident: $data_ty:ty ),*| -> $ret_ty:ty $block:block
	) => (

		pub struct $name;

		impl $crate::routes::Catcher for $name {

			fn check(
				&self,
				$check_req: &$crate::header::RequestHeader,
				$check_res: &$crate::header::ResponseHeader
			) -> bool { $check_block }

			fn validate_data(&self, data: &$crate::Data) {
				$(
					assert!(data.exists::<$data_ty>());
				)*
			}

			fn call<'a>(
				&'a self,
				req: $crate::Request,
				res: $crate::Response,
				raw_data: &'a $crate::Data
			) -> $crate::util::PinnedFuture<'a, $crate::Result<$crate::Response>> {

				use $crate::into::IntoRouteResult;

				async fn catcher(
					mut $req: $crate::Request,
					mut $res: $crate::Response,
					$( $data: &$data_ty ),*
				) -> $ret_ty {
					$block
				}

				$crate::util::PinnedFuture::new(async move {
					catcher(
						req,
						res,
						$(
							raw_data.get::<$data_ty>().unwrap()
						),*
					).await.into_route_result()
				})
			}

		}

	)
}