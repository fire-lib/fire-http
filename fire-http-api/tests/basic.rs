use fire_http_api as fire_api;

use fire_api::error::{ApiError, Error as ErrorTrait, StatusCode};
use fire_api::testing::FirePitApi;
use fire_api::{api, Method, Request};

use std::fmt;

use serde::{Deserialize, Serialize};

use tracing_test::traced_test;

#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
	Internal(String),
	Request(String),
}

impl ApiError for Error {
	fn internal<E: ErrorTrait>(error: E) -> Self {
		Self::Internal(error.to_string())
	}

	fn request<E: ErrorTrait>(error: E) -> Self {
		Self::Request(error.to_string())
	}

	fn status_code(&self) -> StatusCode {
		match self {
			Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::Request(_) => StatusCode::BAD_REQUEST,
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestReq {
	hi: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResp {
	ho: String,
}

impl Request for TestReq {
	type Response = TestResp;
	type Error = Error;

	const PATH: &'static str = "/api/test";
	const METHOD: Method = Method::POST;
}

#[api(TestReq)]
async fn test(r: TestReq) -> Result<TestResp, Error> {
	Ok(TestResp { ho: r.hi })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserReq {
	image_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResp {
	id: String,
	name: String,
	image_size: Option<u32>,
}

impl Request for UserReq {
	type Response = UserResp;
	type Error = Error;

	const PATH: &'static str = "/api/user/{id}";
	const METHOD: Method = Method::GET;
}

#[api(UserReq)]
async fn user(req: UserReq, id: &String) -> Result<UserResp, Error> {
	Ok(UserResp {
		id: id.clone(),
		name: "John".into(),
		image_size: req.image_size,
	})
}

async fn init() -> FirePitApi {
	let mut server = fire::build("127.0.0.1:0").await.unwrap();

	server.add_route(test);
	server.add_route(user);

	let fire = server.build().await.unwrap();
	FirePitApi::new(fire.pit())
}

#[traced_test]
#[tokio::test]
async fn test_test() {
	let pit = init().await;

	let resp = pit.request(&TestReq { hi: "hey".into() }).await.unwrap();
	assert_eq!(resp, TestResp { ho: "hey".into() });
}

#[traced_test]
#[tokio::test]
async fn test_user() {
	let pit = init().await;

	let resp = pit
		.request_with_uri("/api/user/123", &UserReq { image_size: None })
		.await
		.unwrap();
	assert_eq!(
		resp,
		UserResp {
			id: "123".into(),
			name: "John".into(),
			image_size: None
		}
	);

	let resp = pit
		.request_with_uri(
			"/api/user/123",
			&UserReq {
				image_size: Some(42),
			},
		)
		.await
		.unwrap();
	assert_eq!(
		resp,
		UserResp {
			id: "123".into(),
			name: "John".into(),
			image_size: Some(42)
		}
	);
}
