extern crate capnpc;

fn main() {
	::capnpc::compile("logformat", &["src/schema.capnp"]).unwrap();
}
