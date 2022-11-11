use crate::Request;
use crate::ApiError;

use std::time::Duration;
use std::any::{Any, TypeId};
use std::mem::ManuallyDrop;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

use tracing::error;

use fire::{Response, Body, Data};
use fire::header::{RequestHeader, HeaderValues, Method, StatusCode, Mime};
use fire::error::{ServerErrorKind};

#[derive(Debug, Clone)]
pub struct ResponseHeaders(HeaderValues);

impl ResponseHeaders {
	#[doc(hidden)]
	pub fn new() -> Self {
		Self(HeaderValues::new())
	}
}

impl Deref for ResponseHeaders {
	type Target = HeaderValues;

	fn deref(&self) -> &HeaderValues {
		&self.0
	}
}

impl DerefMut for ResponseHeaders {
	fn deref_mut(&mut self) -> &mut HeaderValues {
		&mut self.0
	}
}

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
	res: Result<(ResponseHeaders, Body), R::Error>
) -> fire::Result<Response> {
	let (status, headers, body) = match res {
		Ok((headers, body)) => (StatusCode::OK, headers.0, body),
		Err(e) => {
			error!("request handle error: {:?}", e);

			let body = Body::serialize(&e)
				.map_err(|e| fire::Error::new(
					ServerErrorKind::InternalServerError,
					e
				))?;

			(e.status_code(), HeaderValues::new(), body)
		}
	};

	let mut resp = Response::builder()
		.status_code(status)
		.content_type(Mime::JSON)
		.body(body);
	*resp.values_mut() = headers;

	Ok(resp.build())
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

fn is_resp<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<ResponseHeaders>()
}

fn is_data<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<Data>()
}

/// fn to check if a type can be accessed in a route as reference
#[inline]
pub fn valid_route_data_as_ref<T: Any, R: Any>(data: &Data) -> bool {
	is_req::<T, R>() || is_header::<T>() || is_resp::<T>() || is_data::<T>() ||
	data.exists::<T>()
}

/// fn to check if a type can be accessed in a route as mutable reference
#[inline]
pub fn valid_route_data_as_mut<T: Any, R: Any>(_data: &Data) -> bool {
	is_req::<T, R>() || is_resp::<T>()
}

/// fn to check if a type can be accessed in a route as mutable reference
#[inline]
pub fn valid_route_data_as_owned<T: Any, R: Any>(_data: &Data) -> bool {
	is_req::<T, R>()
}

#[doc(hidden)]
pub struct DataManager<T> {
	inner: RefCell<Option<T>>
}

impl<T> DataManager<T> {
	pub fn new(val: T) -> Self {
		Self {
			inner: RefCell::new(Some(val))
		}
	}

	/// ## Panics
	/// if the value is already taken or borrowed
	#[inline]
	pub fn take(&self) -> T {
		self.inner.borrow_mut().take().unwrap()
	}

	/// ## Panics
	/// If the values is already taken or borrowed mutably
	#[inline]
	pub fn as_ref(&self) -> &T {
		let r = self.inner.borrow();
		let r = ManuallyDrop::new(r);
		// since the borrow counter does not get decreased because of the
		// ManuallyDrop and the lifetime not getting expanded this is safe
		unsafe {
			&*(r.deref().deref() as *const Option<T>)
		}.as_ref().unwrap()
	}

	/// ## Panics
	/// If the values is already taken or borrowed mutably
	#[inline]
	pub fn as_mut(&self) -> &mut T {
		let r = self.inner.borrow_mut();
		let mut r = ManuallyDrop::new(r);
		// since the borrow counter does not get decreased because of the
		// ManuallyDrop and the lifetime not getting expanded this is safe
		unsafe {
			&mut *(r.deref_mut().deref_mut() as *mut Option<T>)
		}.as_mut().unwrap()
	}

	/// ##Panics
	/// if the value was taken previously
	#[inline]
	pub fn take_owned(mut self) -> T {
		self.inner.get_mut().take().unwrap()
	}
}

#[inline]
pub fn get_route_data_as_ref<'a, T: Any, R: Any>(
	data: &'a Data,
	header: &'a RequestHeader,
	req: &'a DataManager<R>,
	resp: &'a DataManager<ResponseHeaders>
) -> &'a T {
	if is_req::<T, R>() {
		let req = req.as_ref();
		<dyn Any>::downcast_ref(req).unwrap()
	} else if is_header::<T>() {
		<dyn Any>::downcast_ref(header).unwrap()
	} else if is_resp::<T>() {
		let resp = resp.as_ref();
		<dyn Any>::downcast_ref(resp).unwrap()
	} else if is_data::<T>() {
		<dyn Any>::downcast_ref(data).unwrap()
	} else {
		data.get::<T>().unwrap()
	}
}

#[inline]
pub fn get_route_data_as_mut<'a, T: Any, R: Any>(
	_data: &'a Data,
	_header: &'a RequestHeader,
	req: &'a DataManager<R>,
	resp: &'a DataManager<ResponseHeaders>
) -> &'a mut T {
	if is_req::<T, R>() {
		let req = req.as_mut();
		<dyn Any>::downcast_mut(req).unwrap()
	} else if is_resp::<T>() {
		let resp = resp.as_mut();
		<dyn Any>::downcast_mut(resp).unwrap()
	} else {
		unreachable!()
	}
}

#[inline]
pub fn get_route_data_as_owned<T: Any, R: Any>(
	_data: &Data,
	_header: &RequestHeader,
	req: &DataManager<R>,
	_resp: &DataManager<ResponseHeaders>
) -> T {
	if is_req::<T, R>() {
		let req = req.take();
		unsafe {
			transform_owned::<T, R>(req)
		}
	} else {
		unreachable!()
	}
}

/// Safety you need to know that T is `R`
#[inline]
pub(crate) unsafe fn transform_owned<T: Any + Sized, R: Any>(from: R) -> T {
	let mut from = ManuallyDrop::new(from);
	(&mut from as *mut ManuallyDrop<R> as *mut T).read()
}