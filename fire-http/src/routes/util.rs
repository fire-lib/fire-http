use super::{path_params::ParamsNames, PathParams};
use crate::{Request, Resources};

use std::any::{Any, TypeId};

fn is_req<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<Request>()
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
pub fn valid_route_data_as_ref<T: Any>(
	name: &str,
	params: &ParamsNames,
	data: &Resources,
) -> bool {
	is_req::<T>()
		|| is_data::<T>()
		|| is_path_params::<T>()
		|| (params.exists(name) && is_string::<T>())
		|| data.exists::<T>()
}

/// fn to check if a type can be accessed in a route as mutable reference
#[inline]
pub fn valid_route_data_as_mut<T: Any>(
	_name: &str,
	_params: &ParamsNames,
	_data: &Resources,
) -> bool {
	is_req::<T>()
}

#[inline]
pub fn get_route_data_as_ref<'a, T: Any>(
	name: &str,
	req: &mut Option<&'a mut Request>,
	params: &'a PathParams,
	data: &'a Resources,
) -> &'a T {
	if is_req::<T>() {
		let req = req.take().unwrap();
		<dyn Any>::downcast_ref(req).unwrap()
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
pub fn get_route_data_as_mut<'a, T: Any>(
	_name: &str,
	req: &mut Option<&'a mut Request>,
	_params: &'a PathParams,
	_data: &'a Resources,
) -> &'a mut T {
	assert!(is_req::<T>());

	let req = req.take().unwrap();
	<dyn Any>::downcast_mut(req).unwrap()
}
