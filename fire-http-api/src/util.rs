use crate::response::ResponseSettings;
use crate::ApiError;
use crate::Request;

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use fire::routes::ParamsNames;
use tracing::error;

use fire::error::ServerErrorKind;
use fire::header::{HeaderValues, Method, Mime, RequestHeader};
use fire::routes::PathParams;
use fire::{Body, Resources, Response};

pub fn setup_request<R: Request>(
	req: &mut fire::Request,
) -> Result<(), R::Error> {
	req.set_size_limit(Some(R::SIZE_LIMIT));
	req.set_timeout(Some(Duration::from_secs(R::TIMEOUT as u64)));

	// check headers
	let headers_missing = R::HEADERS
		.iter()
		.any(|key| req.header().value(*key).is_none());
	if headers_missing {
		return Err(R::Error::request(format!(
			"some headers are missing {:?}",
			R::HEADERS
		)));
	}

	Ok(())
}

pub async fn deserialize_req<R: Request + Send + 'static>(
	req: &mut fire::Request,
) -> Result<R, R::Error> {
	// since a get request does not have a body let's parse the query parameters
	if R::METHOD == Method::GET {
		req.deserialize_query()
			.map_err(|e| R::Error::request(format!("malformed request {e}")))
	} else {
		req.deserialize()
			.await
			.map_err(|e| R::Error::request(format!("malformed request {e}")))
	}
}

pub fn serialize_resp<R: Request>(
	resp: &R::Response,
) -> Result<Body, R::Error> {
	Body::serialize(resp)
		.map_err(|e| R::Error::internal(format!("malformed response {e}")))
}

/// todo find a better name
pub fn transform_body_to_response<R: Request>(
	res: Result<(ResponseSettings, Body), R::Error>,
) -> fire::Result<Response> {
	let (status, headers, body) = match res {
		Ok((settings, body)) => (settings.status, settings.headers, body),
		Err(e) => {
			error!("request handle error: {:?}", e);

			let body = Body::serialize(&e).map_err(|e| {
				fire::Error::new(ServerErrorKind::InternalServerError, e)
			})?;

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
	TypeId::of::<T>() == TypeId::of::<ResponseSettings>()
}

fn is_data<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<Resources>()
}

fn is_path_params<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<PathParams>()
}

fn is_string<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<String>()
}

/// fn to check if a type can be accessed in a route as reference
#[inline]
pub fn valid_route_data_as_ref<T: Any, R: Any>(
	name: &str,
	params: &ParamsNames,
	data: &Resources,
) -> bool {
	is_req::<T, R>()
		|| is_header::<T>()
		|| is_resp::<T>()
		|| is_data::<T>()
		|| is_path_params::<T>()
		|| (params.exists(name) && is_string::<T>())
		|| data.exists::<T>()
}

/// fn to check if a type can be accessed in a route as mutable reference
#[inline]
pub fn valid_route_data_as_mut<T: Any, R: Any>(
	_name: &str,
	_params: &ParamsNames,
	_data: &Resources,
) -> bool {
	is_req::<T, R>() || is_resp::<T>()
}

/// fn to check if a type can be accessed in a route as mutable reference
#[inline]
pub fn valid_route_data_as_owned<T: Any, R: Any>(
	_name: &str,
	_params: &ParamsNames,
	_data: &Resources,
) -> bool {
	is_req::<T, R>()
}

#[doc(hidden)]
pub struct DataManager<T> {
	inner: RefCell<Option<T>>,
}

impl<T> DataManager<T> {
	pub fn new(val: T) -> Self {
		Self {
			inner: RefCell::new(Some(val)),
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
		unsafe { &*(r.deref().deref() as *const Option<T>) }
			.as_ref()
			.unwrap()
	}

	/// ## Panics
	/// If the values is already taken or borrowed mutably
	#[inline]
	pub fn as_mut(&self) -> &mut T {
		let r = self.inner.borrow_mut();
		let mut r = ManuallyDrop::new(r);
		// since the borrow counter does not get decreased because of the
		// ManuallyDrop and the lifetime not getting expanded this is safe
		unsafe { &mut *(r.deref_mut().deref_mut() as *mut Option<T>) }
			.as_mut()
			.unwrap()
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
	name: &str,
	req: &'a DataManager<R>,
	header: &'a RequestHeader,
	params: &'a PathParams,
	resp: &'a DataManager<ResponseSettings>,
	data: &'a Resources,
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
	} else if is_path_params::<T>() {
		<dyn Any>::downcast_ref::<T>(params).unwrap()
	} else if params.exists(name) && is_string::<T>() {
		<dyn Any>::downcast_ref::<T>(params.get(name).unwrap()).unwrap()
	} else {
		data.get::<T>().unwrap()
	}
}

#[inline]
pub fn get_route_data_as_mut<'a, T: Any, R: Any>(
	_name: &str,
	req: &'a DataManager<R>,
	_header: &'a RequestHeader,
	_params: &'a PathParams,
	resp: &'a DataManager<ResponseSettings>,
	_data: &'a Resources,
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
pub fn get_route_data_as_owned<'a, T: Any, R: Any>(
	_name: &str,
	req: &'a DataManager<R>,
	_header: &'a RequestHeader,
	_params: &'a PathParams,
	_resp: &'a DataManager<ResponseSettings>,
	_data: &'a Resources,
) -> T {
	if is_req::<T, R>() {
		let req = req.take();
		unsafe { transform_owned::<T, R>(req) }
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
