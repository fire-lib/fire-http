use std::fmt;
use std::str::Utf8Error;

use tracing::{error, warn};

#[doc(hidden)]
pub use hyper::upgrade;

use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite;
use tungstenite::protocol::Role;

// rexport
use tungstenite::protocol::Message as ProtMessage;
pub use tungstenite::{
	error::Error,
	protocol::CloseFrame,
	protocol::frame::coding::CloseCode
};

use futures_util::stream::StreamExt;
use futures_util::sink::SinkExt;

use sha1::Digest;

// See: https://github.com/hyperium/hyper/blob/master/examples/upgrades.rs

// we first need to implement raw routes

/*
// data might be needed to be cloned
ws_route!(MyRoute, "path", |ws, data| {
	// this is spawned in a new task
	// anything happens here
});
*/

// Basic Route
/// Creates a WebSocket route
///
/// Because this spawns a new task it will clone every used data
#[macro_export]
macro_rules! ws_route {
	(
		$name:ident, $path:expr,
		|$ws:ident| $block:block
	) => (
		$crate::ws_route!($name, $path, |$ws,| -> () $block);
	);
	(
		$name:ident, $path:expr,
		|$ws:ident| -> $ret_ty:ty $block:block
	) => (
		$crate::ws_route!($name, $path, |$ws,| -> $ret_ty $block);
	);
	(
		$name:ident, $path:expr,
		|$ws:ident, $($data:ident: $data_ty:ty),*| -> $ret_ty:ty $block:block
	) => (
		pub struct $name;

		impl $crate::routes::RawRoute for $name {
			fn check(
				&self,
				req: &$crate::routes::HyperRequest
			) -> bool {
				req.method() == $crate::header::Method::GET &&
				$crate::routes::check_static(req.uri().path(), $path)
			}

			fn validate_data(&self, data: &$crate::Data) {
				$(
					assert!(data.exists::<$data_ty>());
				)*
			}

			fn call<'a>(
				&'a self,
				req: &'a mut $crate::routes::HyperRequest,
				raw_data: &'a $crate::Data
			) -> $crate::util::PinnedFuture<'a,
				Option<$crate::Result<$crate::Response>>
			> {

				async fn handler(
					mut $ws: $crate::ws::WebSocket,
					$( $data: $data_ty ),*
				) -> $ret_ty {
					$block
				}

				$crate::util::PinnedFuture::new(async move {
					// if headers not match for websocket
					// return bad request
					let header_upgrade = req.headers()
						.get("upgrade")
						.and_then(|v| v.to_str().ok());
					let header_version = req.headers()
						.get("sec-websocket-version")
						.and_then(|v| v.to_str().ok());
					let websocket_key = req.headers()
						.get("sec-websocket-key")
						.map(|v| v.as_bytes());

					if !matches!(
						(header_upgrade, header_version, websocket_key),
						(Some("websocket"), Some("13"), Some(_))
					) {
						return Some(Err(
							$crate::error::ClientErrorKind::BadRequest.into()
						))
					}

					// calculate websocket key stuff
					// unwrap does not fail because we check above
					let websocket_key = websocket_key.unwrap();
					let ws_accept = $crate::ws::ws_accept(websocket_key);

					$(
						let $data = raw_data.get::<$data_ty>().unwrap().clone();
					)*

					let on_upgrade = $crate::ws::upgrade::on(req);

					// we need to spawn a future because
					// upgrade on can only be fulfilled after
					// we send SWITCHING_PROTOCOLS
					tokio::task::spawn(async move {
						match on_upgrade.await {
							Ok(upgraded) => {
								let ws = $crate::ws::WebSocket::new(
									upgraded
								).await;

								let ret = handler(
									ws,
									$( $data ),*
								).await;

								$crate::ws::log_websocket_return(ret);
							},
							Err(e) => $crate::ws::upgrade_error(e)
						}
					});


					Some(Ok(
						$crate::Response::builder()
							.status_code(
								$crate::header::StatusCode::SWITCHING_PROTOCOLS
							)
							.header("connection", "upgrade")
							.header("upgrade", "websocket")
							.header("sec-websocket-accept", ws_accept)
							.build()
					))
				})
			}
		}

	)
}


/// we need to expose this instead of inlining it in the macro since
/// tracing logs the crate name and we wan't it to be associated with
/// fire http instead of the crate that uses the macro
#[doc(hidden)]
pub fn upgrade_error(e: hyper::Error) {
	error!("websocket upgrade error {:?}", e);
}

pub trait LogWebSocketReturn: fmt::Debug {
	fn should_log_error(&self) -> bool;
}

impl<T, E> LogWebSocketReturn for Result<T, E>
where
	T: fmt::Debug,
	E: fmt::Debug
{
	fn should_log_error(&self) -> bool {
		self.is_err()
	}
}

impl LogWebSocketReturn for () {
	fn should_log_error(&self) -> bool {
		false
	}
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
pub fn ws_accept(key: &[u8]) -> String {
	let mut sha1 = sha1::Sha1::new();
	sha1.update(key);
	sha1.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
	// cannot fail because 
	base64::encode(sha1.finalize())
}

#[cfg(feature = "json")]
macro_rules! try2 {
	($e:expr) => (match $e {
		Some(v) => v,
		None => return Ok(None)
	})
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Message {
	Text(String),
	Binary(Vec<u8>)
}

impl Message {
	pub fn into_data(self) -> Vec<u8> {
		match self {
			Self::Text(t) => t.into(),
			Self::Binary(b) => b
		}
	}

	pub fn to_text(&self) -> Result<&str, Utf8Error> {
		match self {
			Self::Text(t) => Ok(&t),
			Self::Binary(b) => std::str::from_utf8(b)
		}
	}
}

impl From<String> for Message {
	fn from(s: String) -> Self {
		Self::Text(s)
	}
}

impl From<&str> for Message {
	fn from(s: &str) -> Self {
		Self::Text(s.into())
	}
}

impl From<Vec<u8>> for Message {
	fn from(v: Vec<u8>) -> Self {
		Self::Binary(v)
	}
}

impl From<&[u8]> for Message {
	fn from(v: &[u8]) -> Self {
		Self::Binary(v.into())
	}
}

impl From<Message> for ProtMessage {
	fn from(m: Message) -> Self {
		match m {
			Message::Text(t) => Self::Text(t),
			Message::Binary(b) => Self::Binary(b)
		}
	}
}


#[derive(Debug)]
pub struct WebSocket {
	inner: WebSocketStream<upgrade::Upgraded>
}

impl WebSocket {

	pub async fn new(upgraded: upgrade::Upgraded) -> Self {
		Self {
			inner: WebSocketStream::from_raw_socket(
				upgraded,
				Role::Server,
				None
			).await
		}
	}

	// used for tests
	#[doc(hidden)]
	pub fn from_raw(
		inner: WebSocketStream<upgrade::Upgraded>
	) -> Self {
		Self { inner }
	}

	/// Handles Ping and Pong messages
	/// 
	/// never returns Error::ConnectionClose | Error::AlreadyClosed
	pub async fn receive(&mut self) -> Result<Option<Message>, Error> {
		// loop used to handle Message::Pong | Message::Ping
		loop {
			let res = self.inner.next().await.transpose();
			return match res {
				Ok(None) => Ok(None),
				Ok(Some(ProtMessage::Text(t))) => Ok(Some(Message::Text(t))),
				Ok(Some(ProtMessage::Binary(b))) => {
					Ok(Some(Message::Binary(b)))
				},
				Ok(Some(ProtMessage::Ping(d))) => {
					// respond with a pong
					self.inner.send(ProtMessage::Pong(d)).await?;
					// then listen for a new message
					continue
				},
				Ok(Some(ProtMessage::Pong(_))) => continue,
				Ok(Some(ProtMessage::Close(_))) => Ok(None),
				Ok(Some(ProtMessage::Frame(f))) => {
					warn!("we received a websocket frame {:?}", f);
					// Todod should we do something about this frame??
					continue
				},
				Err(Error::ConnectionClosed) |
				Err(Error::AlreadyClosed) => Ok(None),
				Err(e) => Err(e)
			};
		}
	}

	pub async fn send<M>(&mut self, msg: M) -> Result<(), Error>
	where M: Into<Message> {
		self.inner.send(msg.into().into()).await
	}

	pub async fn close(&mut self, code: CloseCode, reason: String) {
		let _ = self.inner.send(ProtMessage::Close(Some(CloseFrame {
			code, reason: reason.into()
		}))).await;
		let _ = self.inner.close(None).await;
		// close is close
		// don't mind if you could send close or not
	}

	pub async fn ping(&mut self) -> Result<(), Error> {
		self.inner.send(ProtMessage::Ping(vec![])).await
	}

	/// calls receive and then deserialize
	#[cfg(feature = "json")]
	pub async fn deserialize<D>(&mut self) -> Result<Option<D>, JsonError>
	where D: serde::de::DeserializeOwned {
		let msg = try2!(self.receive().await?).into_data();
		serde_json::from_slice(&msg)
			.map(|d| Some(d))
			.map_err(|e| e.into())
	}

	/// calls serialize then send
	#[cfg(feature = "json")]
	pub async fn serialize<S: ?Sized>(&mut self, value: &S) -> Result<(), JsonError>
	where S: serde::Serialize {
		let v = serde_json::to_string(value)?;
		self.send(v).await
			.map_err(|e| e.into())
	}

}

#[cfg(feature = "json")]
mod json_error {

	use super::Error;
	use std::fmt;

	#[derive(Debug)]
	pub enum JsonError {
		ConnectionError(Error),
		SerdeError(serde_json::Error)
	}

	impl From<Error> for JsonError {
		fn from(e: Error) -> Self {
			Self::ConnectionError(e)
		}
	}

	impl From<serde_json::Error> for JsonError {
		fn from(e: serde_json::Error) -> Self {
			Self::SerdeError(e)
		}
	}

	impl fmt::Display for JsonError {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			fmt::Debug::fmt(self, f)
		}
	}

	impl std::error::Error for JsonError {
		fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
			match self {
				Self::ConnectionError(e) => Some(e),
				Self::SerdeError(e) => Some(e)
			}
		}
	}
}

#[cfg(feature = "json")]
pub use json_error::JsonError;