
use crate::into::IntoResponse;

use std::fmt;
use std::path::Path;
use std::convert::AsRef;
use std::io::SeekFrom;
use std::pin::Pin;
use std::task::{ Context, Poll };
use std::convert::TryInto;

use tokio::{io, fs};
use io::AsyncSeekExt;

use http::header::{ RequestHeader, StatusCode, Mime };
use http::{Response, Body};

#[derive(Debug, Clone)]
pub struct Range {
	pub start: usize,
	pub end: Option<usize>
}

impl Range {

	pub fn parse(header: &RequestHeader) -> Option<Self> {

		let range = header.value("range")?;
		if !range.starts_with("bytes=") {
			return None
		}

		let mut range = range[6..].split('-');

		let start: usize = range.next()?
			.parse().ok()?;

		let end = range.next()?;
		let end: Option<usize> = if end != "" {
			Some(end.parse().ok()?)
		} else {
			None
		};

		Some(Self { start, end })
	}

	/*pub async fn into_response( self, path: PathBuf ) -> Result<Response> {

		let file = try_not_found!( File::open( path ).await );
		let metadata = try_not_found!( file.metadata().await );
		let size = metadata.len();
		let start = self.start;
		let end = self.end.unwrap_or( size - 1 );

		if end < 0 || end >= size || start > end {
			return Err( Error::NotFound )
		}

		// apache no-gzip
		// content type

	}*/

}

pub struct PartialFile {
	file: fs::File,
	mime_type: Mime,
	size: usize,
	start: usize,
	end: usize
}

impl PartialFile {

	/*pub fn new<M>( file: fs::File, mime_type: M, range: Range ) -> Self
	where M: Into<Mime> {
		Self { file, mime_type: mime_type.into(), range }
	}*/

	/// returns not found if path is directory
	pub async fn open<P>(path: P, range: Range) -> io::Result<Self>
	where P: AsRef<Path> {

		let extension = path.as_ref()
			.extension()
			.and_then(|f| f.to_str());

		let mime_type = match extension {
			Some(e) => Mime::from_ext(e),
			None => Mime::Binary
		};

		let mut file = fs::File::open( path ).await?;
		let metadata = file.metadata().await?;

		// make sure we open a file
		if !metadata.is_file() {
			return Err(io::Error::new(io::ErrorKind::NotFound, "expected file found folder"))
		}

		let size: usize = metadata.len().try_into()
			.map_err(|_| io::Error::new(io::ErrorKind::NotFound, "file to large"))?;
		let start = range.start;
		let end = range.end.unwrap_or( size - 1 );

		if end >= size || start >= end {
			return Err(io::Error::new(io::ErrorKind::Other, RangeIncorrect(range)))
		}

		file.seek( SeekFrom::Start(start as u64) ).await?;

		// apache no-gzip
		// content type

		Ok( Self {
			file, mime_type,
			size, start, end
		} )
	}

}


#[derive(Debug)]
pub struct RangeIncorrect(Range);

impl fmt::Display for RangeIncorrect {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

impl std::error::Error for RangeIncorrect {}


// TODO NEED TO CHANGE u64
// A File which streams a range
pub struct FixedFile {
	file: fs::File,
	read: usize,
	len: usize
}

impl FixedFile {
	pub fn new( file: fs::File, len: usize ) -> Self {
		Self {
			file, len,
			read: 0
		}
	}
}

impl io::AsyncRead for FixedFile {
	fn poll_read(
		mut self: Pin<&mut Self>,
		cx: &mut Context,
		buf: &mut io::ReadBuf<'_>
	) -> Poll<io::Result<()>> {

		// if finished ready
		if self.read >= self.len {
			debug_assert!(self.read == self.len);
			return Poll::Ready(Ok(()))
		}

		// how much can we read??
		// max self.buf.len()
		// remaining self.len - self.read
		let remaining = self.len - self.read;

		// there is less to write then we have buffer capacity
		// we need to use a new ReadBuf
		if remaining < buf.remaining() {
			// we need to initialize the remaining
			// could be made better
			// but needs unsafe
			// TODO do it
			let n_slice = buf.initialize_unfilled_to(remaining);
			let mut n_buf = io::ReadBuf::new(n_slice);
			debug_assert!(n_buf.filled().len() == 0);

			// read first data
			let res = Pin::new(&mut self.file).poll_read(cx, &mut n_buf);

			// calculate how much was read
			let read = n_buf.filled().len();

			self.read += read;
			buf.advance(read);

			res
		} else {
			// we can use the normal buffer
			let before_len = buf.filled().len();

			// read first data
			let res = Pin::new(&mut self.file).poll_read(cx, buf);

			// calculate how much was read
			let read = buf.filled().len() - before_len;
			self.read += read;

			res
		}
	}
}


impl IntoResponse for PartialFile {
	fn into_response( self ) -> Response {

		let len = (self.end + 1) - self.start;

		let response = Response::builder()
			.status_code(StatusCode::PartialContent)
			.content_type(self.mime_type)
			.header("Accept-Ranges", "bytes")
			.header("Content-Length", len)
			.header("Content-Range", format!( "bytes {}-{}/{}", self.start, self.end, self.size ));

		// if we can read to the end
		// return every thing after self.start
		if self.end + 1 == self.size {
			response.body(Body::from_async_read(self.file))
		} else {
			let fixed_file = FixedFile::new(self.file, len);
			response.body(Body::from_async_read(fixed_file))
		}
		// response
		.build()
	}
}