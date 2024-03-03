use codegen::get;
use fire::Request;
use fire_http_codegen as codegen;

struct MyType;

/// Really awesome function
#[get("/")]
async fn hello_word(_my_ty: &MyType, _req: &mut Request) -> &'static str {
	"Hello, Word!"
}
