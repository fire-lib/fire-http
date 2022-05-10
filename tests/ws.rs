
use fire_http as fire;

use fire::ws::{WebSocket, CloseCode, Error};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol::Role;

#[macro_use]
mod util;

type Data = ();


macro_rules! ws_client {
	($srv_addr:expr, $uri:expr, |$ws:ident| $block:block) => (
		let req = hyper::Request::builder()
			.uri(format!("http://{}{}", $srv_addr, $uri))
			.header("upgrade", "websocket")
			.header("sec-websocket-version", "13")
			.header("sec-websocket-key", "123")
			.body(hyper::Body::empty())
			.unwrap();

		let res = hyper::Client::new().request(req).await
			.expect("could not make websocket request");
		assert_eq!(res.status().as_u16(), 101, "didn't receive switching protocols");
		let upgrade_header = res.headers().get("upgrade").map(|v| v.to_str().ok()).flatten();
		assert_eq!(upgrade_header, Some("websocket"), "header: upgrade != \"websocket\"");

		let upgraded = hyper::upgrade::on(res).await
			.expect("could not upgrade connection");

		let mut $ws = WebSocket::from_raw(
			WebSocketStream::from_raw_socket(
				upgraded,
				Role::Client,
				None
			).await
		);
		let _ = async move { $block }.await;
	)
}


#[tokio::test]
async fn build_con() {

	// TODO improve test
	// to look if we get connection closed properly
	// handle errors in task

	// // build route
	fire::ws_route!(WebSocketRoute, "/", |ws| -> Result<(), Error> {

		let mut c = 0;

		while let Some(msg) = ws.receive().await? {
			// read
			assert_eq!(msg.to_text().unwrap(), format!("Hey {}", c));
			c += 1;
			// send them
			ws.send("Hi").await?;
		}

		println!("connection closed");

		Ok(())

	}, |ret| {
		ret.expect("ws error");
	});

	// builder server
	let addr = spawn_server!(|builder| {
		builder.add_raw_route(WebSocketRoute);
	});

	// make request
	ws_client!(addr, "/", |ws| {
		for i in 0..5 {
			ws.send(format!("Hey {}", i)).await
				.expect("could not send");
			let msg = ws.receive().await
				.expect("could not receive")
				.expect("no message received");
			assert_eq!(msg.to_text().expect("not text"), "Hi");
		}
		ws.close(CloseCode::Normal, "".into()).await;
	});

	// close the connection properly
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;

}