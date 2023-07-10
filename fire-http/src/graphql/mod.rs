use crate::{Request, Response, Error, Data, Body};
use crate::header::{self, RequestHeader, Method, StatusCode, Mime};
use crate::routes::Route;
use crate::util::PinnedFuture;
use crate::error::{ClientErrorKind};

use juniper::{
	RootNode, GraphQLType, GraphQLTypeAsync, GraphQLSubscriptionType,
	ScalarValue
};
use juniper::http::{GraphQLRequest, GraphQLBatchRequest};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphiQl {
	uri: &'static str,
	graphql_uri: &'static str
}

impl GraphiQl {
	pub const fn new(uri: &'static str, graphql_uri: &'static str) -> Self {
		Self { uri, graphql_uri }
	}
}

impl Route for GraphiQl {
	fn check(&self, header: &RequestHeader) -> bool {
		header.method() == &Method::GET &&
		header.uri().path().starts_with(self.uri)
	}

	fn validate_data(&self, _data: &Data) {}

	fn call<'a>(
		&'a self,
		_req: &'a mut Request,
		_: &'a Data
	) -> PinnedFuture<'a, crate::Result<Response>> {
		PinnedFuture::new(async move {
			Ok(Response::html(
				juniper::http::graphiql::graphiql_source(self.graphql_uri, None)
			))
		})
	}
}

/// This only supports POST requests
pub struct GraphQl<Ctx, Q, M, Sub, S>
where
	Q: GraphQLType<S, Context=Ctx>,
	M: GraphQLType<S, Context=Ctx>,
	Sub: GraphQLType<S, Context=Ctx>,
	S: ScalarValue
{
	uri: &'static str,
	root_node: RootNode<'static, Q, M, Sub, S>,
	context: Ctx
}

impl<Ctx, Q, M, Sub, S> GraphQl<Ctx, Q, M, Sub, S>
where
	Q: GraphQLType<S, Context=Ctx>,
	M: GraphQLType<S, Context=Ctx>,
	Sub: GraphQLType<S, Context=Ctx>,
	S: ScalarValue
{
	pub fn new(
		uri: &'static str,
		root_node: RootNode<'static, Q, M, Sub, S>,
		context: Ctx
	) -> Self {
		Self { uri, root_node, context }
	}
}

impl<Ctx, Q, M, Sub, S> Route for GraphQl<Ctx, Q, M, Sub, S>
where
	Q: GraphQLTypeAsync<S, Context=Ctx> + Send,
	Q::TypeInfo: Send + Sync,
	M: GraphQLTypeAsync<S, Context=Ctx> + Send,
	M::TypeInfo: Send + Sync,
	Sub: GraphQLSubscriptionType<S, Context=Ctx> + Send,
	Sub::TypeInfo: Send + Sync,
	Ctx: Send + Sync,
	S: ScalarValue + Send + Sync
{
	fn check(&self, header: &RequestHeader) -> bool {
		header.method() == &Method::POST &&
		header.uri().path().starts_with(self.uri)
	}

	fn validate_data(&self, _data: &Data) {}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		_: &'a Data
	) -> PinnedFuture<'a, crate::Result<Response>> {
		PinnedFuture::new(async move {
			// get content-type of request
			let content_type = req.header().value(header::CONTENT_TYPE)
				.unwrap_or("");

			let req: GraphQLBatchRequest<S> = match content_type {
				"application/json" => {
					// read json
					req.deserialize().await?
				},
				"application/graphql" => {
					let body = req.body.take().into_string().await
						.map_err(Error::from_client_io)?;

					GraphQLBatchRequest::Single(
						GraphQLRequest::new(body, None, None)
					)
				},
				_ => return Err(ClientErrorKind::BadRequest.into())
			};

			let res = req.execute(&self.root_node, &self.context).await;

			let mut resp = Response::builder()
				.content_type(Mime::JSON);

			if !res.is_ok() {
				resp = resp.status_code(StatusCode::BAD_REQUEST);
			}

			Ok(resp.body(Body::serialize(&res).unwrap()).build())
		})
	}
}