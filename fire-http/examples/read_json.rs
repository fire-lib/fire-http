use fire_http as fire;

use fire::{Result, Response, get, post};
use fire::header::Mime;

use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct MyType {
	crazy: String,
	good: String
}

get!{ HelloWorld, "/",
	|_r| -> Response {
		Response::builder()
			.content_type(Mime::HTML)
			.body(
		"<a id=\"btn\" style=\"color: blue\">Send MyType</a>
		<script>
			const myType = {
				crazy: 'crazy',
				good: 'good'
			};
			document.getElementById('btn').addEventListener('click', async e => {
				const res = await fetch('/', {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify(myType)
				}).then(r => r.text());
				alert(`response: ${ await res }`);
			});
		</script>" )
			.build()
	}
}

post!{ HelloWorldJson, "/",
	|req| -> Result<String> {
		let my_type: MyType = req.deserialize().await?;
		Ok(format!("read type {:?}", my_type))
	}
}

#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000").await
		.expect("Address could not be parsed");

	server.add_route( HelloWorld );
	server.add_route( HelloWorldJson );

	server.light().await.unwrap();
}