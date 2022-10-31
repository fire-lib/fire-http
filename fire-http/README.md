A simple http server library.

## Example
```rust no_run
# use fire_http as fire;
use fire::get;

struct GlobalName(String);

// handle a simple get request
#[get("/")]
fn root(global_name: &GlobalName) -> String {
	format!("Hi, this is {}", global_name.0)
}

#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000").await
		.expect("Failed to parse address");

	server.add_data(GlobalName("fire".into()));
	server.add_route(root);

	server.ignite().await.unwrap();
}
```

For more examples look in the examples directory and the test directory.

## Features
- json
- fs
<!-- - http2 (enables http 2 support) -->
- ws (adds websocket support)
- trace