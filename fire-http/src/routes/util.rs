use crate::{Request, Data};

use std::any::{Any, TypeId};


fn is_req<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<Request>()
}

fn is_data<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<Data>()
}

/// fn to check if a type can be accessed in a route as reference
#[inline]
pub fn valid_route_data_as_ref<T: Any>(data: &Data) -> bool {
	is_req::<T>() || is_data::<T>() || data.exists::<T>()
}

/// fn to check if a type can be accessed in a route as mutable reference
#[inline]
pub fn valid_route_data_as_mut<T: Any>(_data: &Data) -> bool {
	is_req::<T>()
}

#[inline]
pub fn get_route_data_as_ref<'a, T: Any>(
	data: &'a Data,
	req: &mut Option<&'a mut Request>
) -> &'a T {
	if is_req::<T>() {
		let req = req.take().unwrap();
		<dyn Any>::downcast_ref(req).unwrap()
	} else if is_data::<T>() {
		<dyn Any>::downcast_ref(data).unwrap()
	} else {
		data.get::<T>().unwrap()
	}
}

#[inline]
pub fn get_route_data_as_mut<'a, T: Any>(
	_data: &'a Data,
	req: &mut Option<&'a mut Request>
) -> &'a mut T {
	assert!(is_req::<T>());

	let req = req.take().unwrap();
	<dyn Any>::downcast_mut(req).unwrap()
}