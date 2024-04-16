use core::fmt;
use std::convert::Infallible;
use std::error::Error as StdError;
use std::str::FromStr;
use std::{future::Future, ops::Deref};

use crate::error::{ClientErrorKind, ErrorKind};
use crate::{
	routes::{ParamsNames, PathParams},
	state::State,
	Request, Resources,
};

pub trait Extractor<'a> {
	type Error: ExtractorError;

	fn validate_requirements(
		name: &str,
		params: &ParamsNames,
		resources: &Resources,
	);

	fn prepare(
		_name: &str,
		_req: &mut Request,
		_params: &PathParams,
		_state: &mut State,
		_resources: &Resources,
	) -> impl Future<Output = Result<(), Self::Error>> {
		async { Ok(()) }
	}

	fn extract(
		name: &str,
		request: &mut Option<&'a mut Request>,
		params: &'a PathParams,
		state: &'a State,
		resources: &'a Resources,
	) -> Result<Self, Self::Error>
	where
		Self: Sized;
}

pub trait ExtractorError: StdError + Send + Sync {
	fn error_kind(&self) -> ErrorKind;

	fn into_std(self) -> Box<dyn StdError + Send + Sync>;
}

impl ExtractorError for Infallible {
	fn error_kind(&self) -> ErrorKind {
		unreachable!()
	}

	fn into_std(self) -> Box<dyn StdError + Send + Sync> {
		unreachable!()
	}
}

pub struct Res<'a, T: ?Sized>(&'a T);

impl<'a, T> Extractor<'a> for Res<'a, T>
where
	T: Send + Sync + 'static,
{
	type Error = Infallible;

	fn validate_requirements(_: &str, _: &ParamsNames, resources: &Resources) {
		resources.get::<T>().unwrap();
	}

	fn extract(
		_name: &str,
		_req: &mut Option<&'a mut Request>,
		_params: &'a PathParams,
		_state: &'a State,
		resources: &'a Resources,
	) -> Result<Self, Self::Error>
	where
		Self: Sized,
	{
		Ok(Res(resources.get::<T>().unwrap()))
	}
}

impl<T> Deref for Res<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0
	}
}

impl<'a> Extractor<'a> for &'a mut Request {
	type Error = Infallible;

	fn validate_requirements(_: &str, _: &ParamsNames, _: &Resources) {}

	fn extract(
		_name: &str,
		request: &mut Option<&'a mut Request>,
		_params: &'a PathParams,
		_state: &'a State,
		_resources: &'a Resources,
	) -> Result<Self, Self::Error>
	where
		Self: Sized,
	{
		Ok(request.take().unwrap())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathParam<T>(T);

impl<'a, T> Extractor<'a> for PathParam<T>
where
	T: Send + Sync + FromStr + 'a,
	T::Err: StdError + Send + Sync + 'static,
{
	type Error = PathError<T::Err>;

	fn validate_requirements(name: &str, names: &ParamsNames, _: &Resources) {
		if !names.exists(name) {
			panic!("Path parameter `{}` does not exist", name);
		}
	}

	fn extract(
		name: &str,
		_req: &mut Option<&'a mut Request>,
		params: &'a PathParams,
		_state: &'a State,
		_resources: &'a Resources,
	) -> Result<Self, Self::Error>
	where
		Self: Sized,
	{
		params.parse(name).map(PathParam).map_err(PathError)
	}
}

impl<T> Deref for PathParam<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Debug)]
pub struct PathError<T>(pub T);

impl<T> fmt::Display for PathError<T>
where
	T: fmt::Display,
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Failed to parse path parameter: {}", self.0)
	}
}

impl<T> StdError for PathError<T>
where
	T: StdError + 'static,
{
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		Some(&self.0)
	}
}

impl<T> ExtractorError for PathError<T>
where
	T: StdError + Send + Sync + 'static,
{
	fn error_kind(&self) -> ErrorKind {
		ErrorKind::Client(ClientErrorKind::BadRequest)
	}

	fn into_std(self) -> Box<dyn StdError + Send + Sync> {
		Box::new(self.0)
	}
}

impl<'a> Extractor<'a> for &'a Resources {
	type Error = Infallible;

	fn validate_requirements(_: &str, _: &ParamsNames, _: &Resources) {}

	fn extract(
		_name: &str,
		_req: &mut Option<&'a mut Request>,
		_params: &'a PathParams,
		_state: &'a State,
		resources: &'a Resources,
	) -> Result<Self, Self::Error>
	where
		Self: Sized,
	{
		Ok(resources)
	}
}
