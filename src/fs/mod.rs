
use crate::http::Response;
use crate::into::IntoResponse;

use tokio::{io, fs};

use http::body::BodyWithTimeout;

use std::path::{ Path, PathBuf };
use std::convert::AsRef;
use std::str::Utf8Error;
use std::fmt;

use percent_encoding::percent_decode_str;

mod file;
pub use file::File;

mod partial_file;
pub use partial_file::{PartialFile, Range};

pub mod caching;
pub use caching::Caching;

pub mod static_files;
pub use static_files::{StaticFilesRoute, StaticFileRoute, serve_file};

/// returns io::Error not found if path is directory
pub async fn with_file<P>(path: P) -> io::Result<Response>
where P: AsRef<Path> {
	File::open(path).await
		.map(|f| f.into_response())
}

/// returns io::Error not found if path is directory
pub async fn with_partial_file<P>(path: P, range: Range) -> io::Result<Response>
where P: AsRef<Path> {
	PartialFile::open(path, range).await
		.map(|pf| pf.into_response())
}

/// Static get handler which servers/returns a file.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::get_with_file;
/// 
/// type Data = ();
/// 
/// get_with_file! { Index, "/" => "./www/index.html" }
/// ```
#[macro_export]
macro_rules! get_with_file {
	($name:ident, $($tt:tt)*) => (
		$crate::get_with_file!($name<Data>, $($tt)*);
	);
	($name:ident<$data_ty:ty>, $uri:expr => $path:expr) => (
		$crate::get!(
			$name<$data_ty>,
			$uri,
			|_r| -> $crate::Result<$crate::http::Response> {
				$crate::fs::with_file($path).await
					.map_err($crate::Error::from_client_io)
			}
		);
	)
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
	body: BodyWithTimeout,
	path: P
) -> io::Result<()>
where P: AsRef<Path> {
	let mut file = fs::File::create(path).await?;
	body.copy_to_async_write(&mut file).await
}