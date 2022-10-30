use crate::{Response, Body};
use crate::into::IntoResponse;

use tokio::{io, fs};

use std::fmt;
use std::path::{Path, PathBuf};
use std::convert::AsRef;
use std::str::Utf8Error;

use percent_encoding::percent_decode_str;


mod file;
pub use file::File;

mod partial_file;
pub use partial_file::{PartialFile, Range};

mod caching;
pub use caching::Caching;

mod static_files;
pub use static_files::{StaticFiles, StaticFile, serve_file};

/// returns io::Error not found if the path is a directory
pub async fn with_file<P>(path: P) -> io::Result<Response>
where P: AsRef<Path> {
	File::open(path).await
		.map(|f| f.into_response())
}

/// returns io::Error not found if the path is a directory
pub async fn with_partial_file<P>(path: P, range: Range) -> io::Result<Response>
where P: AsRef<Path> {
	PartialFile::open(path, range).await
		.map(|pf| pf.into_response())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntoPathBufError {
	TraversalAttack,
	InvalidCharacter,
	Utf8(Utf8Error)
}

impl fmt::Display for IntoPathBufError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

impl std::error::Error for IntoPathBufError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Utf8(u) => Some(u),
			_ => None
		}
	}
}

impl From<Utf8Error> for IntoPathBufError {
	fn from(e: Utf8Error) -> Self {
		Self::Utf8(e)
	}
}

pub trait IntoPathBuf {
	fn into_path_buf(self) -> Result<PathBuf, IntoPathBufError>;
}

impl IntoPathBuf for &str {
	fn into_path_buf(self) -> Result<PathBuf, IntoPathBufError> {
		let mut path_buf = PathBuf::new();

		for (i, part) in self.split('/').enumerate() {
			match (i, part) {
				(0, "") => continue,
				(_, "..") => { path_buf.pop(); },
				(_, ".") => continue,
				(_, p) => {

					let dec = percent_decode_str(p)
						.decode_utf8()?;

					if dec.contains('\\') ||
						dec.contains('/') ||
						dec.starts_with('.') {
						return Err(IntoPathBufError::InvalidCharacter)
					}

					path_buf.push(dec.as_ref());

				}
			}
		}

		Ok(path_buf)
	}
}


// TODO maybe return crate::Result
// because file::create should be a server Error
pub async fn write_body_to_file<P>(
	body: Body,
	path: P
) -> io::Result<()>
where P: AsRef<Path> {
	let mut file = fs::File::create(path).await?;
	let reader = body.into_async_reader();
	tokio::pin!(reader);

	// body.copy_to_async_write(&mut file).await
	io::copy(&mut reader, &mut file).await
		.map(|_| ())
}