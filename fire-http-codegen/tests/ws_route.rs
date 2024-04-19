use codegen::ws;
use fire_http_codegen as codegen;

use fire::{impl_res_extractor, ws::WebSocket};

#[derive(Debug, Clone)]
struct SomeStruct;

impl_res_extractor!(SomeStruct);

#[ws("/ws")]
async fn handler(_ws: WebSocket, _other: &SomeStruct) -> () {
	()
}

#[tokio::test]
async fn insert_handler() {
	let mut fire = fire::build("127.0.0.1:0").await.unwrap();

	fire.add_data(SomeStruct);
	fire.add_raw_route(handler);
}
