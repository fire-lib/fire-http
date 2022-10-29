use std::{fs, env};
use std::path::Path;
use std::fmt::Write;


fn main() {
	let out_dir = env::var("OUT_DIR").unwrap();

	generate_mime(&out_dir);
}

// (const, canHaveCharsetUtf8, [types..], [extensions..])
const MIMES: &[(&str, bool, &[&str], &[&str])] = &[
	// text
	("TEXT", true, &["text/plain"], &["txt"]),
	("HTML", true, &["text/html"], &["html"]),
	("JS", true, &["application/javascript"], &["js"]),
	("CSS", true, &["text/css"], &["css"]),
	("JSON", true, &["application/json"], &["json"]),
	("CSV", true, &["text/csv"], &["csv"]),
	("DOC", false, &["application/msword"], &["doc"]),
	("DOCX", false,
		&["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
		&["docx"]
	),
	("PDF", false, &["application/pdf"], &["pdf"]),
	("PHP", true, &["application/php"], &["php"]),
	("RTF", false, &["application/rtf"], &["rtf"]),
	("SH", false, &["application/x-sh"], &["sh"]),
	("VSD", false, &["application/vnd.visio"], &["vsd"]),
	("XML", true, &["text/xml"], &["xml"]),

	// imgs
	("JPG", false, &["image/jpeg"], &["jpg"]),
	("PNG", false, &["image/png"], &["png"]),
	("GIF", false, &["image/gif"], &["gif"]),
	("SVG", false, &["image/svg+xml"], &["svg"]),
	("ICO", false, &["image/vnd.microsoft.icon"], &["ico"]),
	("TIFF", false, &["image/tiff"], &["tiff"]),
	("WBP", false, &["image/webp"], &["webp"]),

	// fonts
	("EOT", false, &["application/vnd.ms-fontobject"], &["eot"]),
	("TTF", false, &["font/ttf"], &["ttf"]),
	("WOFF", false, &["font/woff"], &["woff"]),
	("WOOF2", false, &["font/woff2"], &["woff2"]),

	// video
	("AVI", false, &["video/x-msvideo"], &["avi"]),
	("OGV", false, &["video/ogg"], &["ogv"]),
	("WEBM", false, &["video/webm"], &["webm"]),
	("MP4", false, &["video/mp4"], &["mp4"]),

	// audio
	("AAC", false, &["audio/aac"], &["aac"]),
	("MP3", false, &["audio/mpeg"], &["mp3"]),
	("OGA", false, &["audio/ogg"], &["oga"]),
	("WAV", false, &["audio/wav"], &["wav"]),
	("WEBA", false, &["audio/webm"], &["weba"]),

	// Archives
	("RAR", false, &["application/vnd.rar"], &["rar"]),
	("TAR", false, &["application/x-tar"], &["tar"]),
	("ZIP", false, &["application/zip"], &["zip"]),
	("_7ZIP", false, &["application/x-7z-compressed"], &["7z"]),

	// Binary
	("JAR", false, &["application/java-archive"], &["jar"]),
	("BINARY", false, &["application/octet-stream"], &["bin"]),
	("WASM", false, &["application/wasm"], &["wasm"])
];

fn generate_mime(out_dir: &str) {
	let dest_path = Path::new(&out_dir).join("mime.rs");

	let mut value_enum = format!(
		"\
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
		enum MimeValue {{\n"
	);

	let mut mime_consts = String::new();
	let mut from_extension_fn = format!(
		"\tfn from_extension(e: &str) -> Option<Self> {{\n\
		\t\tmatch e {{\n"
	);
	let mut from_str_fn = format!(
		"\tfn from_str(s: &str) -> Option<Self> {{\n\
		\t\tmatch s {{\n"
	);
	let mut extension_fn = format!(
		"\tfn extension(&self) -> &'static str {{\n\
		\t\tmatch self {{\n"
	);
	let mut as_str_fn = format!(
		"\tfn as_str(&self) -> &'static str {{\n\
		\t\tmatch self {{\n"
	);
	let mut as_str_with_maybe_charset = format!(
		"\tfn as_str_with_maybe_charset(&self) -> &'static str {{\n\
		\t\tmatch self {{\n"
	);

	for (id, utf8, types, extensions) in MIMES {
		write!(value_enum,
			"\t{id},\n"
		).unwrap();

		write!(mime_consts,
			"\t/// File extensions: {}  \n\
			\t/// mime types: {}\n\
			\tpub const {id}: Self = Self(MimeValue::{id});\n",
			extensions.join(", "),
			types.join(", ")
		).unwrap();

		for ext in *extensions {
			write!(from_extension_fn,
				"\t\t\t\"{ext}\" => Some(Self::{id}),\n"
			).unwrap();
		}

		for ty in *types {
			write!(from_str_fn,
				"\t\t\t\"{ty}\" => Some(Self::{id}),\n"
			).unwrap();

			if *utf8 {
				write!(from_str_fn,
					"\t\t\t\"{ty}; charset=utf-8\" => Some(Self::{id}),\n"
				).unwrap();
			}
		}

		let ext = extensions.first().unwrap();
		write!(extension_fn,
			"\t\t\tSelf::{id} => \"{ext}\",\n"
		).unwrap();

		let ty = types.first().unwrap();
		write!(as_str_fn,
			"\t\t\tSelf::{id} => \"{ty}\",\n"
		).unwrap();

		if *utf8 {
			write!(as_str_with_maybe_charset,
				"\t\t\tSelf::{id} => \"{ty}; charset=utf-8\",\n"
			).unwrap();
		} else {
			write!(as_str_with_maybe_charset,
				"\t\t\tSelf::{id} => \"{ty}\",\n"
			).unwrap();
		}
	}

	// now end functions
	write!(value_enum,
		"}}\n"
	).unwrap();

	write!(from_extension_fn,
		"\t\t\t_ => None\n\
		\t\t}}\n\
		\t}}\n"
	).unwrap();

	write!(from_str_fn,
		"\t\t\t_ => None\n\
		\t\t}}\n\
		\t}}\n"
	).unwrap();

	write!(extension_fn,
		"\t\t}}\n\
		\t}}\n"
	).unwrap();

	write!(as_str_fn,
		"\t\t}}\n\
		\t}}\n"
	).unwrap();

	write!(as_str_with_maybe_charset,
		"\t\t}}\n\
		\t}}\n"
	).unwrap();

	// impl value enum
	let content = format!(
		"{value_enum}\n\
		impl MimeValue {{\n\
		{from_extension_fn}\n\
		{from_str_fn}\n\
		{extension_fn}\n\
		{as_str_fn}\n\
		{as_str_with_maybe_charset}\
		}}\n\n\
		impl Mime {{\n\
		{mime_consts}\
		}}"
	);

	fs::write(dest_path, content).unwrap();
}