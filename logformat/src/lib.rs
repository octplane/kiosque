extern crate capnp;

pub mod schema_capnp {
  include!(concat!(env!("OUT_DIR"), "/schema_capnp.rs"));
}
