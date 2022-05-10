
use std::convert::TryFrom;
use std::net::SocketAddr;

use hyper_http::method::Method as HMethod;
use hyper_http::uri::{
	Uri as HUri,
	Scheme as HScheme,
	Authority as HAuthority
};
use hyper_http::version::Version as HVersion;
use hyper_http::status::StatusCode as HStatusCode;
use hyper_http::header::{
	HeaderMap as HHeaderMap,
	HeaderValue as HHeaderValue
};
use hyper_http::response::Parts as HParts;

use http::header::{ RequestHeader, Method, Version, Uri, HeaderValues };
use http::header::{ ResponseHeader, StatusCode };


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
	Method(HMethod),
	NoHost,
	HostInvalid,
	Uri,
	Version(HVersion),
	// used for RequestBuilder
	Unknown
}


type Result<T> = std::result::Result<T, HeaderError>;


pub fn convert_hyper_parts_to_fire_header(
	parts: hyper_http::request::Parts,
	address: SocketAddr
) -> Result<RequestHeader> {

	let method = convert_hyper_method_to_fire_method( parts.method )?;
	let uri = convert_hyper_uri_to_fire_uri( parts.uri, &parts.headers )?;
	let version = convert_hyper_version_to_fire_version( parts.version )?;
	let values = convert_hyper_values_to_values( parts.headers )?;

	Ok( RequestHeader { address, method, uri, version, values } )
}


fn convert_hyper_method_to_fire_method(method: HMethod) -> Result<Method> {
	Ok(match method {
		HMethod::GET => Method::Get,
		HMethod::POST => Method::Post,
		HMethod::PUT => Method::Put,
		HMethod::DELETE => Method::Delete,
		HMethod::HEAD => Method::Head,
		HMethod::OPTIONS => Method::Options,
		HMethod::CONNECT => Method::Connect,
		HMethod::PATCH => Method::Patch,
		HMethod::TRACE => Method::Trace,
		_ => return Err(HeaderError::Method(method))
	})
}


fn convert_hyper_uri_to_fire_uri(uri: HUri, headers: &HHeaderMap<HHeaderValue>) -> Result<Uri> {

	// need to replace authority
	let mut parts = uri.into_parts();
	parts.scheme = Some(HScheme::HTTP); // Try to detect it

	// get host infos
	let host = headers.get("host").ok_or(HeaderError::NoHost)?;

	parts.authority = Some(
		HAuthority::try_from(host.as_bytes())
			.map_err(|_| HeaderError::HostInvalid)?
	);

	// hopefully this gets inlined
	let uri = HUri::from_parts(parts)
		.map_err(|_| HeaderError::Uri)?;

	Uri::new(uri).ok_or(HeaderError::Uri)
}


fn convert_hyper_version_to_fire_version(version: HVersion) -> Result<Version> {
	Ok(match version {
		HVersion::HTTP_10 => Version::One,
		HVersion::HTTP_11 => Version::OnePointOne,
		HVersion::HTTP_2 => Version::Two,
		HVersion::HTTP_3 => Version::Three,
		_ => return Err(HeaderError::Version(version))
	})
}

fn convert_hyper_values_to_values( header: HHeaderMap<HHeaderValue> ) -> Result<HeaderValues> {

	Ok(HeaderValues::from_inner(header))

	// let iter: HIntoIter<HHeadVal> = header.into_iter();
	// for (key, value) in iter {

	// 	let key = key.ok_or( HeaderError::HeaderValues )?;
	// 	let key: &str = key.as_ref();
	// 	let key = key.to_string();

	// 	let value = value.to_str()?.to_string();

	// 	values.push( Value::new( key, value ) );
	// }

	// Ok( values )
}


pub fn convert_fire_header_to_hyper_header(parts: &mut HParts, mut header: ResponseHeader) {

	parts.version = convert_fire_version_to_hyper_version(header.version);

	parts.status = convert_fire_status_to_hyper_status(header.status_code);

	let content_type = header.content_type.to_string();
	if !content_type.is_empty() {
		header.values.insert("content-type", content_type);
	}

	// header
	parts.headers = header.values.into_inner();
}

fn convert_fire_version_to_hyper_version(version: Version) -> HVersion {
	match version {
		Version::One => HVersion::HTTP_10,
		Version::OnePointOne => HVersion::HTTP_11,
		Version::Two => HVersion::HTTP_2,
		Version::Three => HVersion::HTTP_3
	}
}

fn convert_fire_status_to_hyper_status(status: StatusCode) -> HStatusCode {
	// only fails if bellow 100 or above 600 (can't happen)
	HStatusCode::from_u16( status.code() ).unwrap()
}