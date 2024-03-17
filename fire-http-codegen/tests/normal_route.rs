use codegen::get;
use fire::Request;
use fire_http_codegen as codegen;

struct MyType;

/// Really awesome function
#[get("/")]
async fn hello_word(_my_ty: &MyType, _req: &mut Request) -> &'static str {
	"Hello, Word!"
}

/// Really awesome function
#[get("/{named}")]
async fn hello_word_2(
	_my_ty: &MyType,
	_named: &String,
	_req: &mut Request,
) -> &'static str {
	"Hello, Word!"
}
