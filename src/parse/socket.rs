
use std::net::{SocketAddr, IpAddr, AddrParseError};

pub type ParseResult<T> = Result<T, AddrParseError>;

pub trait ParseSocketAddr {
	fn parse(self) -> ParseResult<SocketAddr>;
}

impl ParseSocketAddr for &str {
	fn parse(self) -> ParseResult<SocketAddr> {
		self.parse()
	}
}

impl ParseSocketAddr for (&str, u16) {
	fn parse(self) -> ParseResult<SocketAddr> {
		self.0.parse::<IpAddr>()
			.map(|ip| (ip, self.1))
			.map(Into::into)
	}
}

impl ParseSocketAddr for SocketAddr {
	fn parse(self) -> ParseResult<SocketAddr> {
		Ok(self)
	}
}