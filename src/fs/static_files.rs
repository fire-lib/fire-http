
use super::{ with_file, with_partial_file };
use super::{ IntoPathBuf, Caching, Range };

use crate::{ Error, Data };
use crate::routes::{ Route, check_static };
use crate::into::IntoResponse;
use crate::error::ClientErrorKind;
use crate::util::PinnedFuture;
use crate::request::Request;

use std::path::{ Path, PathBuf };
use std::time::Duration;
use std::io;

use http::header::{ RequestHeader, Method };
use http::Response;


/// returns io::Error not found if path is directory
pub async fn serve_file(
	path: impl AsRef<Path>,
	req: &Request,
	caching: Option<Caching>
) -> io::Result<Response> {

	// check caching
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
		caching.complete_header(&mut res.header);
	}

	Ok(res)
}




pub struct StaticFilesRoute {
	uri: &'static str,
	path: PathBuf,
	caching: Option<Caching>
}

impl StaticFilesRoute {

	pub fn new(uri: &'static str, raw_path: &'static str) -> Self {
		Self::prv_new(uri, raw_path, None)
	}

	pub fn new_raw(
		uri: &'static str,
		path: PathBuf,
		caching: Option<Caching>
	) -> Self {
		Self { uri, path, caching }
	}

	fn prv_new(
		uri: &'static str,
		raw_path: &'static str,
		caching: Option<Caching>
	) -> Self {
		let path = PathBuf::from(raw_path);
		Self { uri, path, caching }
	}

	pub fn cache(uri: &'static str, raw_path: &'static str) -> Self {
		Self::prv_new(uri, raw_path, Some(Caching::default()))
	}

	pub fn cache_with_age(
		uri: &'static str,
		raw_path: &'static str,
		max_age: Duration
	) -> Self {
		Self::prv_new(uri, raw_path, Some(Caching::new(max_age)))
	}

}

impl<D: Data> Route<D> for StaticFilesRoute {

	fn check(&self, header: &RequestHeader) -> bool {
		header.method() == &Method::Get &&
		header.uri().path().starts_with(self.uri)
	}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		_: &'a D
	) -> PinnedFuture<'a, crate::Result<Response>> {

		let mut full_path_buf = self.path.clone();
		let uri = self.uri;
		let caching = self.caching.clone();

		PinnedFuture::new(async move {

			let res_path_buf = req.header().uri()
				.path()[uri.len()..]
				.into_path_buf();

			// validate path buf
			// if path is a directory serve_file will return NotFound
			let path_buf = res_path_buf
				.map_err(|e| Error::new(ClientErrorKind::NotFound, e))?;

			// build full pathbuf
			full_path_buf.push(path_buf);

			serve_file(full_path_buf, &req, caching).await
				.map_err(Error::from_client_io)
		})
	}

}


/// Static get handler which servers files from a directory.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use std::time::Duration;
/// use fire::static_files;
/// 
/// type Data = ();
/// 
/// static_files! { Files, "/files" => "./www/" }
/// 
/// #[tokio::main]
/// async fn main() {
/// 	let mut server = fire::build("127.0.0.1:0", ()).unwrap();
/// 	// adds the handler without any caching
/// 	server.add_route(Files::new());
/// 	// adds caching in release builds
/// 	server.add_route(Files::cache());
/// 	// adds caching with customized Max Age in release builds
/// 	server.add_route(Files::cache_with_age(Duration::from_secs(60)));
/// }
/// ```
/// 
/// ## Caching
/// Todo: document caveats
#[macro_export]
macro_rules! static_files {
	($name:ident, $uri:expr => $path:expr) => (

		pub struct $name;

		impl $name {
			pub fn new() -> $crate::fs::StaticFilesRoute {
				$crate::fs::StaticFilesRoute::new($uri, $path)
			}

			// only caches on release
			pub fn cache() -> $crate::fs::StaticFilesRoute {
				if cfg!(debug_assertions) {
					Self::new()
				} else {
					$crate::fs::StaticFilesRoute::cache($uri, $path)
				}
			}

			pub fn cache_with_age(
				max_age: std::time::Duration
			) -> $crate::fs::StaticFilesRoute {
				if cfg!(debug_assertions) {
					Self::new()
				} else {
					$crate::fs::StaticFilesRoute::cache_with_age(
						$uri,
						$path,
						max_age
					)
				}
			}

			pub fn cache_no_age() -> $crate::fs::StaticFilesRoute {
				Self::cache_with_age(std::time::Duration::from_secs(0))
			}

			pub fn cache_always() -> $crate::fs::StaticFilesRoute {
				$crate::fs::StaticFilesRoute::cache($uri, $path)
			}
		}

	)
}


#[derive(Debug, Clone)]
pub struct StaticFileRoute {
	uri: &'static str,
	path: &'static str,
	caching: Option<Caching>
}

impl StaticFileRoute {

	pub fn new(uri: &'static str, path: &'static str) -> Self {
		Self { uri, path, caching: None }
	}

	pub fn cache(uri: &'static str, path: &'static str) -> Self {
		Self { uri, path, caching: Some(Caching::default()) }
	}

	pub fn cache_with_age(
		uri: &'static str,
		path: &'static str,
		max_age: Duration
	) -> Self {
		Self { uri, path, caching: Some(Caching::new(max_age)) }
	}

}

impl<D: Data> Route<D> for StaticFileRoute {

	fn check(&self, header: &RequestHeader) -> bool {
		header.method() == &Method::Get
		&& check_static(header.uri().path(), self.uri)
	}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		_: &D
	) -> PinnedFuture<'a, crate::Result<Response>> {

		let path = self.path;
		let caching = self.caching.clone();

		PinnedFuture::new(async move {
			serve_file(path, &req, caching).await
				.map_err(Error::from_client_io)
		})
	}

}


#[macro_export]
macro_rules! static_file {
	($name:ident, $uri:expr => $path:expr) => (

		pub struct $name;

		impl $name {
			pub fn new() -> $crate::fs::StaticFileRoute {
				$crate::fs::StaticFileRoute::new($uri, $path)
			}

			// only caches on release
			pub fn cache() -> $crate::fs::StaticFileRoute {
				if cfg!(debug_assertions) {
					$crate::fs::StaticFileRoute::new($uri, $path)
				} else {
					$crate::fs::StaticFileRoute::cache($uri, $path)
				}
			}

			pub fn cache_with_age(
				max_age: std::time::Duration
			) -> $crate::fs::StaticFileRoute {
				if cfg!(debug_assertions) {
					$crate::fs::StaticFileRoute::new($uri, $path)
				} else {
					$crate::fs::StaticFileRoute::cache_with_age(
						$uri,
						$path,
						max_age
					)
				}
			}

			pub fn cache_no_age() -> $crate::fs::StaticFileRoute {
				Self::cache_with_age(std::time::Duration::from_secs(0))
			}

			pub fn cache_always() -> $crate::fs::StaticFileRoute {
				$crate::fs::StaticFileRoute::cache($uri, $path)
			}
		}

	)
}

/// Dynamic get request handler which servers a file if a path is provided.
/// 
/// 
/// Can be used if the uri needs to be mapped from a database
/// or for example if a file is only available for certain users.
/// 
/// ## Example
/// ```
/// # use fire_http as fire;
/// use fire::dyn_static_files;
/// 
/// type Data = ();
/// 
/// dyn_static_files! {
/// 	DynamicFiles, "/files/",
/// 	|req| { // needs to return fire::Result<PathBuf>
/// 		unimplemented!()
/// 	}
/// }
/// 
/// ```
#[macro_export]
macro_rules! dyn_static_files {
	($name:ident, $uri:expr, |$req:ident| $block:block) => (
		$crate::dyn_static_files!($name, $uri, self, |$req,| $block);
	);
	($name:ident, $uri:expr, $self:ident, |$req:ident| $block:block) => (
		$crate::dyn_static_files!($name, $uri, $self, |$req,| $block);
	);
	($name:ident, $uri:expr, |$req:ident, $($data:ident),*| $block:block) => (
		$crate::dyn_static_files!($name, $uri, self, |$req, $($data),*| $block);
	);
	(
		$name:ident,
		$uri:expr,
		$self:ident,
		|$req:ident, $($data:ident),*| $block:block
	) => (

		pub struct $name {
			caching: Option<$crate::fs::Caching>
		}

		impl $name {

			pub fn new() -> Self {
				Self { caching: None }
			}

			pub fn cache() -> Self {
				Self {
					caching: match cfg!(debug_assertions) {
						true => None,
						false => Some($crate::fs::Caching::default())
					}
				}
			}

			pub fn cache_with_age(max_age: std::time::Duration) -> Self {
				Self {
					caching: match cfg!(debug_assertions) {
						true => None,
						false => Some($crate::fs::Caching::new(max_age))
					}
				}
			}

			pub fn cache_always() -> Self {
				Self {
					caching: Some($crate::fs::Caching::default())
				}
			}

			pub fn req_uri<'a>(
				&self,
				req: &'a $crate::request::Request
			) -> &'a str {
				&req.header().uri().path()[$uri.len()..]
			}

		}

		impl $crate::routes::Route<Data> for $name {

			fn check(
				&self,
				header: &$crate::http::header::RequestHeader
			) -> bool {
				header.method() == &$crate::http::header::Method::Get
				&& header.uri().path().starts_with($uri)
			}

			fn call<'a>(
				&'a $self,
				$req: &'a mut $crate::request::Request,
				raw_data: &'a Data
			) -> $crate::util::PinnedFuture<'a, $crate::Result<$crate::http::Response>> {

				let caching = $self.caching.clone();

				$crate::util::PinnedFuture::new(async move {

					$(let $data = raw_data.$data();)*

					let path_buf: $crate::Result<std::path::PathBuf> = async {
						$block
					}.await;

					let path_buf = path_buf?;

					$crate::fs::serve_file(path_buf, &$req, caching).await
						.map_err($crate::Error::from_client_io)
				})
			}

		}

	)
}