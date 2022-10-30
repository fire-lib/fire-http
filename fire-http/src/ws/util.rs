use super::{WebSocket, LogWebSocketReturn};
use crate::{Result, Data, Response};
use crate::error::ClientErrorKind;
use crate::header::{
	StatusCode, UPGRADE, SEC_WEBSOCKET_VERSION, SEC_WEBSOCKET_KEY, CONNECTION,
	SEC_WEBSOCKET_ACCEPT
};
use crate::server::HyperRequest;

use std::mem::ManuallyDrop;
use std::any::{Any, TypeId};

use tracing::error;

use sha1::Digest;

use hyper::upgrade::OnUpgrade;

#[doc(hidden)]
pub use tokio::task::spawn;


fn is_ws<T: Any>() -> bool {
	TypeId::of::<T>() == TypeId::of::<WebSocket>()
}

/// fn to check if a type can be accessed in a websocket handler
#[inline]
pub fn valid_ws_data<T: Any>(data: &Data) -> bool {
	is_ws::<T>() || data.exists::<T>()
}

pub trait WebSocketCloneGuard {
	fn clone(&self) -> Self;
}

impl WebSocketCloneGuard for WebSocket {
	fn clone(&self) -> Self { unreachable!() }
}

impl<T: Clone> WebSocketCloneGuard for T {
	fn clone(&self) -> Self {
		Clone::clone(self)
	}
}

/// clones the data, if the data would be the websocket returns None
#[inline]
pub fn prepare_ws_data<T: Any + WebSocketCloneGuard>(data: &Data) -> Option<T> {
	if !is_ws::<T>() {
		Some(data.get::<T>().unwrap().clone())
	} else {
		None
	}
}

#[inline]
pub fn get_ws_data<T: Any>(
	data: Option<T>,
	ws: &mut Option<WebSocket>
) -> T {
	if is_ws::<T>() {
		let ws = ws.take().unwrap();
		unsafe {
			transform_websocket(ws)
		}
	} else {
		data.unwrap()
	}
}

/// Safety you need to know that T is `WebSocket`
unsafe fn transform_websocket<T: Any>(ws: WebSocket) -> T {
	let mut ws = ManuallyDrop::new(ws);
	(&mut ws as *mut ManuallyDrop<WebSocket> as *mut T).read()
}

/// we need to expose this instead of inlining it in the macro since
/// tracing logs the crate name and we wan't it to be associated with
/// fire http instead of the crate that uses the macro
#[doc(hidden)]
pub fn upgrade_error(e: hyper::Error) {
	error!("websocket upgrade error {:?}", e);
}
/// we need to expose this instead of inlining it in the macro since
/// tracing logs the crate name and we wan't it to be associated with
/// fire http instead of the crate that uses the macro
#[doc(hidden)]
pub fn log_websocket_return(r: impl LogWebSocketReturn) {
	if r.should_log_error() {
		error!("websocket connection closed with error {:?}", r);
	}
}

// does the key need to be a specific length?
#[doc(hidden)]
pub fn upgrade(req: &mut HyperRequest) -> Result<(OnUpgrade, String)> {
	// if headers not match for websocket
	// return bad request
	let header_upgrade = req.headers()
		.get(UPGRADE)
		.and_then(|v| v.to_str().ok());
	let header_version = req.headers()
		.get(SEC_WEBSOCKET_VERSION)
		.and_then(|v| v.to_str().ok());
	let websocket_key = req.headers()
		.get(SEC_WEBSOCKET_KEY)
		.map(|v| v.as_bytes());

	if !matches!(
		(header_upgrade, header_version, websocket_key),
		(Some("websocket"), Some("13"), Some(_))
	) {
		return Err(ClientErrorKind::BadRequest.into())
	}

	// calculate websocket key stuff
	// unwrap does not fail because we check above
	let websocket_key = websocket_key.unwrap();
	let ws_accept = {
		let mut sha1 = sha1::Sha1::new();
		sha1.update(websocket_key);
		sha1.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
		// cannot fail because 
		base64::encode(sha1.finalize())
	};

	let on_upgrade = hyper::upgrade::on(req);

	Ok((on_upgrade, ws_accept))
}

#[doc(hidden)]
pub fn switching_protocols(ws_accept: String) -> Response {
	Response::builder()
		.status_code(StatusCode::SWITCHING_PROTOCOLS)
		.header(CONNECTION, "upgrade")
		.header(UPGRADE, "websocket")
		.header(SEC_WEBSOCKET_ACCEPT, ws_accept)
		.build()
}