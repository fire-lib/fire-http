use crate::Request;
use crate::ApiError;

use std::time::Duration;
use std::any::{Any, TypeId};
use std::mem::ManuallyDrop;

use tracing::error;

use fire::{Response, Body, Data};
use fire::header::{RequestHeader, Method, StatusCode, Mime};
use fire::error::{ServerErrorKind};


pub fn setup_request<R: Request>(
	req: &mut fire::Request
) -> Result<(), R::Error> {
	req.set_size_limit(Some(R::SIZE_LIMIT));
	req.set_timeout(Some(Duration::from_secs(R::TIMEOUT as u64)));

	// check headers
	let headers_missing = R::HEADERS.iter().any(|key| {
		req.header().value(*key).is_none()
	});
	if headers_missing {
		return Err(R::Error::request(
			format!("some headers are missing {:?}", R::HEADERS)
		))
	}

	Ok(())
}

pub async fn deserialize_req<R: Request + Send + 'static>(
	req: &mut fire::Request
) -> Result<R, R::Error> {
	// since a get request does not have a body let's just mark the body as null
	if R::METHOD == Method::GET {
		serde_json::from_value(serde_json::Value::Null)
			.map_err(|e| R::Error::request(format!("malformed request {e}")))
	} else {
		req.deserialize().await
			.map_err(|e| R::Error::request(format!("malformed request {e}")))
	}
}

pub fn serialize_resp<R: Request>(
	resp: &R::Response
) -> Result<Body, R::Error> {
	Body::serialize(resp)
		.map_err(|e| R::Error::internal(format!("malformed response {e}")))
}

/// todo find a better name
pub fn transform_body_to_response<R: Request>(
	res: Result<Body, R::Error>
) -> fire::Result<Response> {
	let (status, body) = match res {
		Ok(b) => (StatusCode::OK, b),
		Err(e) => {
			error!("request handle error: {:?}", e);

			let body = Body::serialize(&e)
				.map_err(|e| fire::Error::new(
					ServerErrorKind::InternalServerError,
					e
				))?;

			(e.status_code(), body)
		}
	};

	let resp = Response::builder()
		.status_code(status)
		.content_type(Mime::JSON)
		.body(body)
		.build();

	Ok(resp)
}

#[allow(unused)]
macro_rules! trace {
	($($tt:tt)*) => (
		#[cfg(feature = "trace")]
		{
			tracing::trace!($($tt)*);
		}
	)
}


fn is_req<T: Any, R: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<R>()
}

fn is_header<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<RequestHeader>()
}

fn is_data<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<Data>()
}

/// fn to check if a type can be accessed in a route as reference
#[inline]
pub fn valid_route_data_as_ref<T: Any, R: Any>(data: &Data) -> bool {
	is_req::<T, R>() || is_header::<T>() || is_data::<T>() || data.exists::<T>()
}

/// fn to check if a type can be accessed in a route as mutable reference
#[inline]
pub fn valid_route_data_as_owned<T: Any, R: Any>(_data: &Data) -> bool {
	is_req::<T, R>()
}

#[inline]
pub fn get_route_data_as_ref<'a, T: Any, R: Any>(
	data: &'a Data,
	header: &'a RequestHeader,
	req: &'a mut Option<R>
) -> &'a T {
	if is_req::<T, R>() {
		let req = req.as_ref().unwrap();
		<dyn Any>::downcast_ref(req).unwrap()
	} else if is_header::<T>() {
		<dyn Any>::downcast_ref(header).unwrap()
	} else if is_data::<T>() {
		<dyn Any>::downcast_ref(data).unwrap()
	} else {
		data.get::<T>().unwrap()
	}
}

#[inline]
pub fn get_route_data_as_owned<T: Any, R: Any>(
	_data: &Data,
	_header: &RequestHeader,
	req: &mut Option<R>
) -> T {
	if is_req::<T, R>() {
		let req = req.take().unwrap();
		unsafe {
			transform_owned::<T, R>(req)
		}
	} else {
		unreachable!()
	}
}

/// Safety you need to know that T is `WebSocket`
unsafe fn transform_owned<T: Any + Sized, R: Any>(from: R) -> T {
	let mut from = ManuallyDrop::new(from);
	(&mut from as *mut ManuallyDrop<R> as *mut T).read()
}