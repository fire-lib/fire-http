use codegen::get;
use fire::{extractor::PathParam, impl_res_extractor, Request};
use fire_http_codegen as codegen;

#[allow(dead_code)]
struct MyType;

impl_res_extractor!(MyType);

/// Really awesome function
#[get("/")]
async fn hello_word(_my_ty: &MyType, _req: &mut Request) -> &'static str {
	"Hello, Word!"
}

/// Really awesome function
#[get("/{named}")]
async fn hello_word_2(
	_my_ty: &MyType,
	_named: &PathParam<str>,
	_req: &mut Request,
) -> &'static str {
	"Hello, Word!"
}
