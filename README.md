[![CI](https://github.com/fire-lib/fire-http/actions/workflows/ci.yaml/badge.svg)](https://github.com/fire-lib/fire-http-representation/actions/workflows/ci.yaml)
[![crates.io](https://img.shields.io/crates/v/fire-http)](https://crates.io/crates/fire-http)
[![docs.rs](https://img.shields.io/docsrs/fire-http)](https://docs.rs/fire-http)

A simple http server library.

## Example
```rust no_run
# use fire_http as fire;
use fire::{data_struct, get};

// To access data from request handlers
data_struct! {
	#[derive(Debug)]
	struct Data {
		global_name: String
	}
}

// handle a simple get request
get! {
	Root, "/",
	|_r, global_name| -> String {
		format!("Hi, this is {}", global_name)
	}
}

#[tokio::main]
async fn main() {
	let data = Data {
		global_name: "fire".into()
	};

	let mut server = fire::build("0.0.0.0:3000", data)
		.expect("Failed to parse address");

	server.add_route(Root);

	server.light().await
		.expect("server paniced");
}
```

For more examples look in the examples directory and the test directory.

## Features
- json
- fs
- encdec (adds percent encoding and decoding to header values)
- http2 (enables http 2 support)
- ws (adds websocket support)