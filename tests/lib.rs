extern crate log_archive;
extern crate logformat;
extern crate capnp;


#[cfg(test)]
mod tests {
  use super::*;
  use logformat::schema_capnp::{logline};
  use capnp::message::Builder;
  use capnp::serialize;

  use std::collections::HashMap;


  #[test]
  fn it_works() {

    let mut message = Builder::new_default();
    {
      let mut ranking = message.init_root::<logline::Builder>();
      // 16:16, 28 apr 2016
      let t: u64 = 1461852984000000;
      ranking.set_time(t);
      ranking.set_facility("Plop".into());
      {
        let mut facets_builder = ranking.borrow().init_facets();
        let mut kv = facets_builder.borrow().init_entries(4);

        kv.borrow().get(0).set_key("hostname");
        kv.borrow().get(0).set_value("127.0.0.1");

        let mut props = HashMap::<&str,&str>::new();
        props.insert("hostname","127.0.0.1");
        props.insert("path","/search");
        props.insert("query_string","dog");
        
      }

    }
    let mut vec = Vec::new();
    serialize::write_message(&mut vec, &message);
    assert!(vec.len() == 12, "vec len: {}", vec.len());



  }

}
