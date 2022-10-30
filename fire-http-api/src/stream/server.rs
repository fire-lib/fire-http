use super::stream::{Stream, StreamKind};
use super::message::{Message, MessageData, MessageKind};
use super::streamer::RawStreamer;
use super::error::UnrecoverableError;

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::task::Poll;
use std::future::poll_fn;

use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

use tracing::error;

pub use fire::util::PinnedFuture;
use fire::{Response, Data};
use fire::header::Method;
use fire::routes::{RawRoute, HyperRequest};
use fire::ws::{self, WebSocket, JsonError};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Request {
	action: Cow<'static, str>,
	kind: StreamKind
}

pub trait IntoStreamHandler {
	type Stream: Stream;
	type Handler: StreamHandler;

	fn into_handler(self) -> Self::Handler;
}

pub trait StreamHandler {
	fn validate_data(&self, _data: &Data) {}

	/// every MessageData needs to correspond with the StreamTrait
	/// 
	/// ## Warning
	/// Your not allowed to drop streamer before you return from the function
	/// else that may leed to a busy loop in the StreamServer (todo improve that)
	fn handle<'a>(
		&'a self,
		req: MessageData,
		streamer: RawStreamer,
		data: &'a Data
	) -> PinnedFuture<'a, Result<MessageData, UnrecoverableError>>;
}

pub struct StreamServer {
	uri: &'static str,
	inner: Arc<HashMap<Request, Box<dyn StreamHandler + Send + Sync>>>
}

impl StreamServer {
	pub fn new(uri: &'static str) -> Self {
		Self {
			uri,
			inner: Arc::new(HashMap::new())
		}
	}

	pub fn insert<H>(&mut self, handler: H)
	where
		H: IntoStreamHandler,
		H::Handler: StreamHandler + Send + Sync + 'static
	{
		Arc::get_mut(&mut self.inner).unwrap().insert(
			Request {
				action: H::Stream::ACTION.into(),
				kind: H::Stream::KIND
			},
			Box::new(handler.into_handler())
		);
	}
}

impl RawRoute for StreamServer {
	fn check(&self, req: &HyperRequest) -> bool {
		req.method() == Method::GET &&
		fire::routes::check_static(req.uri().path(), self.uri)
	}

	fn call<'a>(
		&'a self,
		req: &'a mut HyperRequest,
		data: &'a Data
	) -> PinnedFuture<'a, Option<fire::Result<Response>>> {
		PinnedFuture::new(async move {
			let (on_upgrade, ws_accept) = match ws::util::upgrade(req) {
				Ok(o) => o,
				Err(e) => return Some(Err(e))
			};

			let handlers = self.inner.clone();
			let data = data.clone();

			// we need to spawn a future because
			// upgrade on can only be fufilled after
			// we send SWITCHING_PROTOCOLS
			tokio::task::spawn(async move {
				match on_upgrade.await {
					Ok(upgraded) => {
						let ws = WebSocket::new(upgraded).await;

						trace!("connection upgraded");
						
						let res = handle_connection(handlers, ws, data).await;
						if let Err(e) = res {
							error!("websocket connection failed with {:?}", e);
						}

					},
					Err(e) => ws::util::upgrade_error(e)
				}
			});

			Some(Ok(ws::util::switching_protocols(ws_accept)))
		})
	}
}

async fn handle_connection(
	handlers: Arc<HashMap<Request, Box<dyn StreamHandler + Send + Sync>>>,
	mut ws: WebSocket,
	data: Data
) -> Result<(), UnrecoverableError> {
	let mut receivers = Receivers::new();
	let mut senders = Senders::new();
	// data: (Request, MessageData)
	let (close_tx, mut close_rx) = mpsc::channel(10);
	let mut ping_interval = interval(Duration::from_secs(30));

	loop {
		tokio::select! {
			msg = ws.deserialize() => {
				let msg: Message = match msg {
					Ok(None) => return Ok(()),
					Ok(Some(m)) => m,
					Err(JsonError::ConnectionError(e)) => {
						return Err(e.to_string().into())
					},
					Err(JsonError::SerdeError(e)) => {
						error!("could not deserialize message {:?}", e);
						// json error just ignore the message
						continue
					}
				};

				trace!("received message {:?}", msg);

				let req = Request {
					action: msg.action,
					kind: msg.kind.into()
				};

				match msg.kind {
					k @ MessageKind::SenderRequest |
					k @ MessageKind::ReceiverRequest => {
						// no handler
						if !handlers.contains_key(&req) {
							error!("no handler for {:?} found", req);
							ws.serialize(&Message {
								kind: msg.kind.into_close(),
								action: req.action.clone(),
								data: MessageData::null()
							}).await.map_err(|e| e.to_string())?;
							continue
						}

						// we know the handler exists
						let (tx, rx) = mpsc::channel(10);

						let streamer = match req.kind {
							// the client want's to send us data
							StreamKind::Sender => {
								if !senders.insert(req.clone(), tx) {
									// the sender already exist
									// don't create a new handler
									continue
								}
								RawStreamer::receiver(rx)
							},
							// the client want's to receive data from us
							StreamKind::Receiver => {
								if !receivers.insert(req.clone(), rx) {
									// the handler already exists
									continue
								}
								RawStreamer::sender(tx)
							}
						};

						// let's send a success message
						ws.serialize(&Message {
							kind: k,
							action: req.action.clone(),
							data: MessageData::null()
						}).await.map_err(|e| e.to_string())?;

						let data = data.clone();
						let handlers = handlers.clone();
						let msg_data = msg.data;
						let close_tx = close_tx.clone();

						// the first task only catches panics
						// and the seconds starts the handler
						// we could also detect a panic when trying to send
						// or receive via a mpsc channel.
						// but that could lead to multiple close messages being
						// sent when the task succesfully exists
						tokio::spawn(async move {
							let panic_close_tx = close_tx.clone();
							let panic_req = req.clone();

							let r = tokio::spawn(async move {

								let handler = match handlers.get(&req) {
									Some(h) => h,
									None => unreachable!()
								};

								let r = handler.handle(
									msg_data,
									streamer,
									&data
								).await;
								match r {
									Ok(m) => {
										let _ = close_tx.send((req, m)).await;
									},
									Err(e) => {
										error!("stream handler unrecoverable \
											error {:?}", e
										);
										let _ = close_tx.send(
											(req, MessageData::null())
										).await;
									}
								}
							}).await;

							if r.is_err() {
								// some error happened so let's send a close req
								let _ = panic_close_tx.send(
									(panic_req, MessageData::null())
								).await;
							}
						});
						
					},
					MessageKind::SenderMessage => {
						// if a handler is already closed don't do anything
						// since it is guaranteed to get closed via close_tx
						// if a handler does not exist
						// this is a protocol error since you would get a
						// a 
						let _ = senders.send(&req, msg.data).await;
					},
					MessageKind::ReceiverMessage => {
						// we should not receive this message
						// this is a protocol error
					},
					MessageKind::SenderClose => {
						senders.remove(&req);
					},
					MessageKind::ReceiverClose => {
						receivers.remove(&req);
					}
				}
			},
			(req, data) = receivers.recv(), if !receivers.is_empty() => {
				ws.serialize(&Message {
					kind: req.kind.into_kind_message(),
					action: req.action,
					data: data
				}).await.map_err(|e| e.to_string())?;
			},
			_ping = ping_interval.tick() => {
				ws.ping().await
					.map_err(|e| e.to_string())?;
			},
			msg = close_rx.recv() => {
				// cannot fail since we always have a close_tx
				let (req, data) = msg.unwrap();

				match req.kind {
					StreamKind::Sender => {
						if senders.remove(&req) {
							ws.serialize(&Message {
								kind: MessageKind::SenderClose,
								action: req.action,
								data: data
							}).await.map_err(|e| e.to_string())?;
						}
					},
					StreamKind::Receiver => {
						if receivers.remove(&req) {
							ws.serialize(&Message {
								kind: MessageKind::ReceiverClose,
								action: req.action,
								data: data
							}).await.map_err(|e| e.to_string())?;
						}
					}
				}
			}
		}
	}
}

struct Receivers {
	inner: HashMap<Request, mpsc::Receiver<MessageData>>,
	// we use a recv queue to make polling more fair since we poll
	// all futures and check if they have available data and the store everything
	// in the queue
	// the problem here is that we don't return on the first one and always poll
	// every future (which is not great)
	recv_queue: Vec<(Request, MessageData)>
}

impl Receivers {
	pub fn new() -> Self {
		Self {
			inner: HashMap::new(),
			recv_queue: vec![]
		}
	}

	pub fn is_empty(&self) -> bool {
		self.inner.is_empty() && self.recv_queue.is_empty()
	}

	/// if no receivers exist this will wait for every
	/// you should don't call recv when `is_empty` returns `true`
	pub async fn recv(&mut self) -> (Request, MessageData) {
		if let Some(msg) = self.recv_queue.pop() {
			return msg;
		}

		debug_assert!(!self.inner.is_empty(), "will wait for ever");

		poll_fn(|ctx| {
			for (req, rx) in self.inner.iter_mut() {
				match rx.poll_recv(ctx) {
					Poll::Pending => continue,
					Poll::Ready(Some(data)) => {
						self.recv_queue.push((req.clone(), data))
					},
					// todo maybe we should remove those
					// but since the receiver will probably quickly be removed
					// it should not be a problem
					Poll::Ready(None) => continue
				}
			}

			match self.recv_queue.pop() {
				Some(m) => Poll::Ready(m),
				None => Poll::Pending
			}
		}).await
	}

	pub fn insert(
		&mut self,
		req: Request,
		recv: mpsc::Receiver<MessageData>
	) -> bool {
		if self.inner.contains_key(&req) {
			return false
		}

		self.inner.insert(req, recv).is_none()
	}

	pub fn remove(&mut self, req: &Request) -> bool {
		self.inner.remove(req).is_some()
	}
}

struct Senders {
	inner: HashMap<Request, mpsc::Sender<MessageData>>
}

impl Senders {
	pub fn new() -> Self {
		Self {
			inner: HashMap::new()
		}
	}

	pub fn insert(
		&mut self,
		req: Request,
		sender: mpsc::Sender<MessageData>
	) -> bool {
		if self.inner.contains_key(&req) {
			return false
		}

		self.inner.insert(req, sender).is_none()
	}

	pub async fn send(&mut self, req: &Request, data: MessageData) {
		if let Some(sender) = self.inner.get(req) {
			// todo should we send an error here??
			let _ = sender.send(data).await;
		}
	}

	pub fn remove(&mut self, req: &Request) -> bool {
		self.inner.remove(req).is_some()
	}
}