use super::{Stream, Streamer};
use super::message::MessageData;
use super::error::UnrecoverableError;
use super::streamer::RawStreamer;
use crate::util::{DataManager, transform_owned};

use fire::Data;

use std::any::{TypeId, Any};


fn is_req<T: Any, R: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<R>()
}

fn is_streamer<T: Any, R: Stream>() -> bool
where R::Message: 'static {
	TypeId::of::<T>() == TypeId::of::<Streamer<R::Message>>()
}

fn is_data<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<Data>()
}


/// fn to check if a type can be accessed in a websocket handler as reference
#[inline]
pub fn valid_stream_data_as_ref<T, S>(data: &Data) -> bool
where
	T: Any,
	S: Stream + 'static
{
	is_req::<T, S>() || is_streamer::<T, S>() ||
	is_data::<T>() || data.exists::<T>()
}

/// fn to check if a type can be accessed in a websocket handler as owned
#[inline]
pub fn valid_stream_data_as_owned<T, S>(_: &Data) -> bool
where
	T: Any,
	S: Stream + 'static
{
	is_req::<T, S>() || is_streamer::<T, S>()
}

#[inline]
pub fn get_stream_data_as_ref<'a, T, S>(
	data: &'a Data,
	req: &'a DataManager<S>,
	streamer: &'a DataManager<Streamer<S::Message>>
) -> &'a T
where
	T: Any,
	S: Stream + 'static
{
	if is_req::<T, S>() {
		let req = req.as_ref();
		<dyn Any>::downcast_ref(req).unwrap()
	} else if is_streamer::<T, S>() {
		let streamer = streamer.as_ref();
		<dyn Any>::downcast_ref(streamer).unwrap()
	} else if is_data::<T>() {
		<dyn Any>::downcast_ref(data).unwrap()
	} else {
		data.get::<T>().unwrap()
	}
}

#[inline]
pub fn get_stream_data_as_owned<T, S>(
	_data: &Data,
	req: &DataManager<S>,
	streamer: &DataManager<Streamer<S::Message>>
) -> T
where
	T: Any,
	S: Stream + 'static
{
	if is_req::<T, S>() {
		let req = req.take();
		unsafe {
			transform_owned::<T, S>(req)
		}
	} else if is_streamer::<T, S>() {
		let streamer = streamer.take();
		unsafe {
			transform_owned::<T, Streamer<S::Message>>(streamer)
		}
	} else {
		unreachable!()
	}
}

pub fn deserialize_req<S: Stream>(
	msg: MessageData
) -> Result<S, UnrecoverableError> {
	msg.deserialize()
		.map_err(|e| format!("failed to deserialize stream request {e}").into())
}

#[inline]
pub fn transform_streamer<S: Stream>(
	streamer: RawStreamer
) -> Streamer<S::Message> {
	streamer.assign_message()
}

pub fn error_to_data<S: Stream>(
	r: Result<(), S::Error>
) -> Result<MessageData, UnrecoverableError> {
	match r {
		Ok(_) => Ok(MessageData::null()),
		Err(e) => {
			// try to convert the error into a message
			MessageData::serialize(e)
				.map_err(|e| e.to_string().into())
		}
	}
}