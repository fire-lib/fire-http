use super::error::UnrecoverableError;
use super::message::MessageData;
use super::streamer::RawStreamer;
use super::{Stream, Streamer};
use crate::util::{transform_owned, DataManager};

use fire::routes::{ParamsNames, PathParams};
use fire::Data;

use std::any::{Any, TypeId};

fn is_req<T: Any, R: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<R>()
}

fn is_streamer<T: Any, R: Stream>() -> bool
where
	R::Message: 'static,
{
	TypeId::of::<T>() == TypeId::of::<Streamer<R::Message>>()
}

fn is_data<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<Data>()
}

fn is_path_params<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<PathParams>()
}

fn is_string<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<String>()
}

/// fn to check if a type can be accessed in a websocket handler as reference
#[inline]
pub fn valid_stream_data_as_ref<T, S>(
	name: &str,
	params: &ParamsNames,
	data: &Data,
) -> bool
where
	T: Any,
	S: Stream + 'static,
{
	is_req::<T, S>()
		|| is_streamer::<T, S>()
		|| is_data::<T>()
		|| is_path_params::<T>()
		|| (params.exists(name) && is_string::<T>())
		|| data.exists::<T>()
}

/// fn to check if a type can be accessed in a websocket handler as owned
#[inline]
pub fn valid_stream_data_as_owned<T, S>(
	_name: &str,
	_params: &ParamsNames,
	_data: &Data,
) -> bool
where
	T: Any,
	S: Stream + 'static,
{
	is_req::<T, S>() || is_streamer::<T, S>()
}

#[inline]
pub fn get_stream_data_as_ref<'a, T, S>(
	name: &str,
	streamer: &'a DataManager<Streamer<S::Message>>,
	req: &'a DataManager<S>,
	params: &'a PathParams,
	data: &'a Data,
) -> &'a T
where
	T: Any,
	S: Stream + 'static,
{
	if is_req::<T, S>() {
		let req = req.as_ref();
		<dyn Any>::downcast_ref(req).unwrap()
	} else if is_streamer::<T, S>() {
		let streamer = streamer.as_ref();
		<dyn Any>::downcast_ref(streamer).unwrap()
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
pub fn get_stream_data_as_owned<'a, T, S>(
	_name: &str,
	streamer: &'a DataManager<Streamer<S::Message>>,
	req: &'a DataManager<S>,
	_params: &'a PathParams,
	_data: &'a Data,
) -> T
where
	T: Any,
	S: Stream + 'static,
{
	if is_req::<T, S>() {
		let req = req.take();
		unsafe { transform_owned::<T, S>(req) }
	} else if is_streamer::<T, S>() {
		let streamer = streamer.take();
		unsafe { transform_owned::<T, Streamer<S::Message>>(streamer) }
	} else {
		unreachable!()
	}
}

pub fn deserialize_req<S: Stream>(
	msg: MessageData,
) -> Result<S, UnrecoverableError> {
	msg.deserialize()
		.map_err(|e| format!("failed to deserialize stream request {e}").into())
}

#[inline]
pub fn transform_streamer<S: Stream>(
	streamer: RawStreamer,
) -> Streamer<S::Message> {
	streamer.assign_message()
}

pub fn error_to_data<S: Stream>(
	r: Result<(), S::Error>,
) -> Result<MessageData, UnrecoverableError> {
	match r {
		Ok(_) => Ok(MessageData::null()),
		Err(e) => {
			// try to convert the error into a message
			MessageData::serialize(e).map_err(|e| e.to_string().into())
		}
	}
}
