extern crate rustc_serialize;
extern crate openssl;
extern crate hyper;
#[macro_use] extern crate nickel;


use nickel::{Nickel, HttpRouter, JsonBody};

fn main() {
  use hyper::net::Openssl;
  use std::io::Read;

  let ssl = Openssl::with_cert_and_key("assets/server.crt", "assets/server.key").unwrap();
  let mut server = Nickel::new();

  server.utilize(middleware! { |request|
    println!("logging request from middleware! {} {:?}", request.origin.method, request.origin.uri);
  });

  server.options("/services/collector/event/1.0", middleware! { |request, response|
    "youpie"
  });

  server.post("/services/collector/event/1.0", middleware! { |request, response|
    let mut buffer = String::new();
    let _ = request.origin.read_to_string(&mut buffer);
    println!("got {}", buffer);
    "youpie"
  });
  server.listen_https("127.0.0.1:6767", ssl);
}
