
use http::{Response, Body};
use http::header::{StatusCode, Mime};


pub trait IntoResponse {
	fn into_response(self) -> Response;
}

macro_rules! into_response {
	($self:ident: $type:ty $b:block) => ( impl IntoResponse for $type {
		fn into_response( $self ) -> Response { $b }
	} )
}

into_response!(self: Response {self});
into_response!(self: StatusCode {self.into()});
into_response!(self: Body {self.into()});

into_response!(self: &'static str {
	Response::builder()
		.content_type(Mime::Text)
		.body(self)
		.build()
});

into_response!(self: String {
	Response::builder()
		.content_type(Mime::Text)
		.body(self)
		.build()
});

into_response!(self: Vec<u8> {
	Response::builder()
		.content_type(Mime::Binary)
		.body(self)
		.build()
});

into_response!(self: () {
	Body::Empty.into()
});