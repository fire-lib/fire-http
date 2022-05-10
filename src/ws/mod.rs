
use std::str::Utf8Error;

use log_to_stdout::error;

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
	($name:ident, $($tt:tt)* ) => (
		$crate::ws_route!($name<Data>, $($tt)*);
	);
	($name:ident<$data_ty:ty>, $path:expr, |$ws:ident| $block:block ) => (
		$crate::ws_route!($name<$data_ty>, $path, |$ws,| -> () $block);
	);
	($name:ident<$data_ty:ty>, $path:expr, |$ws:ident| -> $ret_type:ty $block:block ) => (
		$crate::ws_route!($name<$data_ty>, $path, |$ws,| -> $ret_type $block);
	);
	($name:ident<$data_ty:ty>, $path:expr, |$ws:ident| -> $ret_type:ty $block:block, |$ret:ident| $ret_block:block ) => (
		$crate::ws_route!($name<$data_ty>, $path, |$ws,| -> $ret_type $block, |$ret| $ret_block);
	);
	($name:ident<$data_ty:ty>, $path:expr, |$ws:ident, $( $data:ident ),*| -> $ret_type:ty $block:block ) => (
		$crate::ws_route!($name<$data_ty>, $path, |$ws, $($data),*| -> $ret_type $block, |ret| { ret });
	);
	($name:ident<$data_ty:ty>, $path:expr, |$ws:ident, $( $data:ident ),*| -> $ret_type:ty $block:block, |$ret:ident| $ret_block:block ) => (

		pub struct $name;

		impl $crate::routes::RawRoute<$data_ty> for $name {

			fn check(&self, req: &$crate::request::HyperRequest) -> bool {
				req.method().as_str() == "GET" &&
				$crate::routes::check_static( req.uri().path(), $path )
			}

			fn call<'a>(
				&'a self,
				req: &'a mut $crate::request::RequestBuilder<'_>,
				raw_data: &'a $data_ty
			) -> $crate::util::PinnedFuture<'a, Option<$crate::Result<$crate::http::Response>>> {

				use $crate::into::IntoRouteResult;

				$crate::util::PinnedFuture::new( async move {

					// allowed to unwrap (will not panic)
					let hyper_req = req.hyper_mut().unwrap();

					// if headers not match for websocket
					// return bad request
					let header_upgrade = hyper_req.headers()
						.get("upgrade")
						.and_then(|v| v.to_str().ok());
					let header_version = hyper_req.headers()
						.get("sec-websocket-version")
						.and_then(|v| v.to_str().ok());
					let websocket_key = hyper_req.headers()
						.get("sec-websocket-key")
						.map(|v| v.as_bytes());

					if !matches!(
						(header_upgrade, header_version, websocket_key),
						(Some("websocket"), Some("13"), Some(k))
					) {
						return Some(Err($crate::error::ClientErrorKind::BadRequest.into()))
					}


					// calculate websocket key stuff
					// unwrap does not fail because we check above
					let websocket_key = websocket_key.unwrap();
					let ws_accept = $crate::ws::ws_accept(websocket_key);

					$( let mut $data = raw_data.$data().clone(); )*

					let on_upgrade = $crate::ws::upgrade::on(hyper_req);


					// we need to spawn a future because
					// upgrade on can only be fufilled after
					// we send SWITCHING_PROTOCOLS
					tokio::task::spawn(async move {
						match on_upgrade.await {
							Ok(upgraded) => {
								let mut $ws = $crate::ws::WebSocket::new(upgraded).await;
								let $ret: $ret_type = async move { $block }.await;
								let _: () = { $ret_block };
							},
							Err(e) => $crate::ws::upgrade_error(e)
						}
					});


					Some(Ok(
						$crate::http::Response::builder()
							.status_code($crate::http::header::StatusCode::SwitchingProtocols)
							.header("connection", "upgrade")
							.header("upgrade", "websocket")
							.header("sec-websocket-accept", ws_accept)
							.build()
						//.into()
					))
				} )
			}

		}

	)
}


#[doc(hidden)]
pub fn upgrade_error(e: hyper::Error) {
	error!("upgrade error {:?}", e);
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
					eprintln!("received frame {:?} ?? what todo?", f);
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