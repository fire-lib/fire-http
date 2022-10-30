use super::{with_file, with_partial_file};
use super::{IntoPathBuf, Caching, Range};

use crate::{Request, Response, Error, Data};
use crate::header::{RequestHeader, Method, StatusCode};
use crate::routes::{Route, check_static};
use crate::into::{IntoResponse, IntoRoute};
use crate::error::ClientErrorKind;
use crate::util::PinnedFuture;

use std::path::Path;
use std::time::Duration;
use std::io;


/// returns io::Error not found if the path is a directory
pub async fn serve_file(
	path: impl AsRef<Path>,
	req: &Request,
	caching: Option<Caching>
) -> io::Result<Response> {

	// check caching and if the etag matches return NOT_MODIFIED
	if matches!(&caching, Some(c) if c.if_none_match(req.header())) {
		return Ok(caching.unwrap().into_response())
	}

	let range = Range::parse(req.header());

	let mut res = match range {
		Some(range) => {
			with_partial_file(path, range).await?
				.into_response()
		},
		None => {
			with_file(path).await?
				.into_response()
		}
	};

	// set etag
	if let Some(caching) = caching {
		if matches!(res.header.status_code,
			StatusCode::OK | StatusCode::NOT_FOUND)
		{
			caching.complete_header(&mut res.header);
		}
	}

	Ok(res)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CachingBuilder {
	None,
	Default,
	MaxAge(Duration)
}

impl From<CachingBuilder> for Option<Caching> {
	fn from(b: CachingBuilder) -> Self {
		match b {
			CachingBuilder::None => None,
			CachingBuilder::Default => Some(Caching::default()),
			CachingBuilder::MaxAge(age) => Some(Caching::new(age))
		}
	}
}

/// Static get handler which servers files from a directory.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::fs::StaticFiles;
/// 
/// const FILES: StaticFiles = StaticFiles::new("/files", "./www/");
/// 
/// #[tokio::main]
/// async fn main() {
/// 	let mut server = fire::build("127.0.0.1:0").await.unwrap();
/// 	server.add_route(FILES);
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticFiles {
	uri: &'static str,
	path: &'static str,
	caching: CachingBuilder
}

impl StaticFiles {
	/// Creates a `StaticFiles` with Default caching settings
	pub const fn new(uri: &'static str, path: &'static str) -> Self {
		Self { uri, path, caching: CachingBuilder::Default }
	}

	pub const fn no_cache(uri: &'static str, path: &'static str) -> Self {
		Self { uri, path, caching: CachingBuilder::None }
	}

	pub const fn cache_with_age(
		uri: &'static str,
		path: &'static str,
		max_age: Duration
	) -> Self {
		Self { uri, path, caching: CachingBuilder::MaxAge(max_age) }
	}
}

impl IntoRoute for StaticFiles {
	type IntoRoute = StaticFilesRoute;

	fn into_route(self) -> StaticFilesRoute {
		StaticFilesRoute {
			uri: self.uri,
			path: self.path,
			caching: self.caching.into()
		}
	}
}

#[doc(hidden)]
pub struct StaticFilesRoute {
	uri: &'static str,
	path: &'static str,
	caching: Option<Caching>
}

impl Route for StaticFilesRoute {
	fn check(&self, header: &RequestHeader) -> bool {
		header.method() == &Method::GET &&
		header.uri().path().starts_with(self.uri)
	}

	fn validate_data(&self, _data: &Data) {}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		_: &'a Data
	) -> PinnedFuture<'a, crate::Result<Response>> {
		let uri = self.uri;
		let caching = self.caching.clone();

		PinnedFuture::new(async move {

			let res_path_buf = req.header().uri()
				.path()[uri.len()..]
				.into_path_buf();

			// validate path buf
			// if path is a directory serve_file will return NotFound
			let path_buf = res_path_buf
				.map_err(|e| Error::new(ClientErrorKind::BadRequest, e))?;

			// build full pathbuf
			let path_buf = Path::new(self.path).join(path_buf);

			tracing::info!("trying to serve {:?}", path_buf);

			serve_file(path_buf, &req, caching).await
				.map_err(Error::from_client_io)
		})
	}

}

/// Static get handler which servers/returns a file.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::fs::StaticFile;
///
/// const INDEX: StaticFile = StaticFile::new("/", "./www/index.html");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticFile {
	uri: &'static str,
	path: &'static str,
	caching: CachingBuilder
}

impl StaticFile {
	/// Creates a `StaticFile` with Default caching settings
	pub const fn new(uri: &'static str, path: &'static str) -> Self {
		Self { uri, path, caching: CachingBuilder::Default }
	}

	pub const fn no_cache(uri: &'static str, path: &'static str) -> Self {
		Self { uri, path, caching: CachingBuilder::None }
	}

	pub const fn cache_with_age(
		uri: &'static str,
		path: &'static str,
		max_age: Duration
	) -> Self {
		Self { uri, path, caching: CachingBuilder::MaxAge(max_age) }
	}
}

impl IntoRoute for StaticFile {
	type IntoRoute = StaticFileRoute;

	fn into_route(self) -> StaticFileRoute {
		StaticFileRoute {
			uri: self.uri,
			path: self.path,
			caching: self.caching.into()
		}
	}
}

#[doc(hidden)]
pub struct StaticFileRoute {
	uri: &'static str,
	path: &'static str,
	caching: Option<Caching>
}

impl Route for StaticFileRoute {
	fn check(&self, header: &RequestHeader) -> bool {
		header.method() == &Method::GET &&
		check_static(header.uri().path(), self.uri)
	}

	fn validate_data(&self, _data: &Data) {}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		_: &'a Data
	) -> PinnedFuture<'a, crate::Result<Response>> {
		let path = self.path;
		let caching = self.caching.clone();

		PinnedFuture::new(async move {
			serve_file(path, &req, caching).await
				.map_err(Error::from_client_io)
		})
	}

}