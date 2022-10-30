use fire_http_codegen as codegen;
use codegen::ws;

use fire::ws::WebSocket;

#[derive(Debug, Clone)]
struct SomeStruct;

#[ws("/ws")]
async fn handler(_ws: WebSocket, _other: SomeStruct) -> () {
	()
}