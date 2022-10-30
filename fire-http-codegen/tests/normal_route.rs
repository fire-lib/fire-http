use fire_http_codegen as codegen;
use fire::Request;
use codegen::get;

struct MyType;

/// Really awesome function
#[get("/")]
async fn hello_word(_my_ty: &MyType, _req: &mut Request) -> &'static str {
	"Hello, Word!"
}