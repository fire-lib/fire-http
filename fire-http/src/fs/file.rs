use crate::{Response, Body};
use crate::header::{Mime, CONTENT_LENGTH};
use crate::into::IntoResponse;

use std::path::Path;
use std::convert::AsRef;

use tokio::{io, fs};


pub struct File {
	file: fs::File,
	mime_type: Mime,
	size: usize
}

impl File {
	pub fn new<M>(file: fs::File, mime_type: M, size: usize) -> Self
	where M: Into<Mime> {
		Self { file, mime_type: mime_type.into(), size }
	}

	/// if the path is a directory
	/// returns io::Error NotFound
	pub async fn open<P>(path: P) -> io::Result<Self>
	where P: AsRef<Path> {
		let extension = path.as_ref()
			.extension()
			.and_then(|f| f.to_str());

		let mime_type = extension
			.and_then(Mime::from_extension)
			.unwrap_or(Mime::BINARY);

		let file = fs::File::open(path).await?;
		let metadata = file.metadata().await?;

		// make sure we open a file
		if !metadata.is_file() {
			return Err(io::Error::new(
				io::ErrorKind::NotFound,
				"expected file found folder"
			))
		}

		let size = metadata.len() as usize;

		Ok(Self { file, mime_type, size })
	}

}

impl IntoResponse for File {
	fn into_response(self) -> Response {
		Response::builder()
			.content_type(self.mime_type)
			.header(CONTENT_LENGTH, self.size)
			.body(Body::from_async_reader(self.file))
			.build()
	}
}