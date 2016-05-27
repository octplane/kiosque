extern crate rustc_serialize;
extern crate openssl;
extern crate hyper;
#[macro_use] extern crate nickel;
extern crate capnp;
extern crate logformat;

use std::collections::HashMap;
use nickel::status::StatusCode;
use nickel::{Nickel, HttpRouter};
use rustc_serialize::json;

use capnp::message::Builder;
use logformat::schema_capnp::{logblock};


#[derive(RustcDecodable, Debug)]
pub struct Event {
  line: String,
  source: String,
  tag: String,
  attrs: Option<HashMap<String,String>>
}

#[derive(RustcDecodable, Debug)]
struct SplunkLine {
  event: Event,
  time: String,
  host: String
}

fn main() {
  use hyper::net::Openssl;
  use std::io::Read;

  let ssl = Openssl::with_cert_and_key("assets/server.crt", "assets/server.key").unwrap();
  let mut server = Nickel::new();
  let mut message = Builder::new_default();
  let mut block = message.init_root::<logblock::Builder>();
  let lines = block.borrow().init_entries(50000);


  server.options("/services/collector/event/1.0", middleware! { |request, response| "Connector is ready" }); 

  //  {"event":{"line":"2016/05/03 15:41:29 \u001b[1;33m[W] Custom config '/data/gogs/conf/app.ini' not found, ignore this if you're running first time\u001b[0m\r","source":"stdout","tag":"73825581fed7"},"time":"1462290089.642521","host":"default"}
  // {"event":{"line":"2016/05/03 15:41:29 \u001b[1;36m[T] Custom path: /data/gogs\u001b[0m\r","source":"stdout","tag":"73825581fed7"},"time":"1462290089.643815","host":"default"}
  // {"event":{"line":"May  5 06:41:42 sshd[29]: Server listening on :: port 22.\r","source":"stdout","tag":"gogs/gogs/hungry_jones/dee5ed93cbb6","attrs":{"location":"home"}},"time":"1462430502.652300","host":"default"}

  server.post("/services/collector/event/1.0", middleware! { |request, response|
    let mut buffer = String::new();
    let _ = request.origin.read_to_string(&mut buffer);
    match json::decode::<SplunkLine>(&buffer) {
      Ok(_) => Ok((StatusCode::Ok, "OK")),
      Err(e) => {
        println!("Parsing failed: {}", e);
        Err((StatusCode::BadRequest, e))
       }
    }
  });
  server.listen_https("127.0.0.1:6767", ssl);
}
