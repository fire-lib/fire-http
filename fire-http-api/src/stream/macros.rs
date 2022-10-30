
/// ```ignore
/// stream_handler! {
/// 	async fn name(req: Request, stream, any_data) -> Result<(), Error> {
/// 		// if the request was StreamKind::Sender use
/// 		// stream.recv()
/// 		// else if StreamKind::Receiver
/// 		// stream.send(msg)
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! stream_handler {
	// handle request without data type
	(
		async fn $name:ident( $($ptt:tt)* ) $($tt:tt)*
	) => (
		$crate::stream_handler! {
			async fn $name<Data>($($ptt)*) $($tt)*
		}
	);
	// handle request without data
	(
		async fn $name:ident<$data_ty:ty>(
			$req:ident: $req_ty:ty,
			$stream:ident
		) $($tt:tt)*
	) => (
		$crate::stream_handler! {
			async fn $name<$data_ty>($req: $req_ty, $stream, ) $($tt)*
		}
	);
	// final handler
	(
		async fn $name:ident<$data_ty:ty>(
			$req:ident: $req_ty:ty,
			$stream:ident,
			$($data:ident),*
		) -> $ret_ty:ty $block:block
	) => (
		#[allow(non_camel_case_types)]
		pub struct $name;

		impl $crate::stream::server::StreamHandler<$data_ty> for $name {

			fn action(&self) -> &'static str {
				use $crate::stream::Stream;
				<$req_ty as Stream>::ACTION
			}

			fn kind(&self) -> $crate::stream::StreamKind {
				use $crate::stream::Stream;
				<$req_ty as Stream>::KIND
			}

			fn handle<'a>(
				&'a self,
				req: $crate::stream::message::MessageData,
				streamer: $crate::stream::streamer::RawStreamer,
				data: &'a $data_ty
			) ->
				$crate::stream::server::PinnedFuture<'a,
					std::result::Result<
						$crate::stream::message::MessageData,
						$crate::stream::error::UnrecoverableError
					>
				>
			{
				use $crate::stream::Stream as __Stream;
				use $crate::stream::Streamer as __Streamer;
				use $crate::stream::message::MessageData as __MessageData;

				type __Message = <$req_ty as __Stream>::Message;
				type __Error = <$req_ty as __Stream>::Error;

				async fn __handle(
					$req: $req_ty,
					#[allow(unused_mut)]
					mut $stream: __Streamer<__Message>,
					#[allow(unused_variables)]
					raw_data: &$data_ty
				) -> std::result::Result<(), __Error> {

					$(
						let $data = raw_data.$data();
					)*

					$block
				}

				$crate::stream::server::PinnedFuture::new(async move {

					let req: $req_ty = req.deserialize()
						.map_err(|e| format!(
							"stream: failed to deserialize request {}",
							e
						))?;

					#[allow(unused_mut)]
					let mut streamer = streamer.assign_message::<__Message>();

					let r = __handle(req, streamer, data).await;
					let msg_data = match r {
						Ok(_) => __MessageData::null(),
						Err(e) => {
							__MessageData::serialize(e)
								.map_err(|e| e.to_string())?
						}
					};

					Ok(msg_data)
				})
			}
		}
	)
}

#[cfg(test)]
mod tests {

	use crate::stream::{Stream, StreamKind};
	use crate::stream_handler;
	use crate::error::{self, ApiError, StatusCode};

	use std::fmt;

	use serde::{Serialize, Deserialize};

	struct Data;

	#[derive(Debug, Serialize, Deserialize)]
	struct SenderRequest {
		hi: u64
	}

	#[derive(Debug, Serialize, Deserialize)]
	struct SenderMessage {
		num: u64
	}

	impl Stream for SenderRequest {
		type Message = SenderMessage;
		type Error = Error;

		const KIND: StreamKind = StreamKind::Sender;
		const ACTION: &'static str = "Hi";
	}

	#[derive(Debug, Serialize)]
	enum Error {
		FailedToSend
	}

	impl fmt::Display for Error {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			fmt::Debug::fmt(self, f)
		}
	}

	impl ApiError for Error {
		fn internal<E: error::Error>(_error: E) -> Self { todo!() }
		fn request<E: error::Error>(_error: E) -> Self { todo!() }

		fn status_code(&self) -> StatusCode { todo!() }
	}

	stream_handler! {
		async fn hi_stream(req: SenderRequest, stream) -> Result<(), Error> {
			stream.send(SenderMessage { num: req.hi }).await
				.map_err(|_| Error::FailedToSend)?;

			Ok(())
		}
	}

}