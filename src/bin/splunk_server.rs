extern crate rustc_serialize;
extern crate openssl;
extern crate hyper;
#[macro_use] extern crate nickel;


use nickel::{Nickel, HttpRouter, JsonBody};

#[derive(RustcDecodable, RustcEncodable)]
struct Person {
  firstname: String,
  lastname:  String,
}

fn main() {
  use hyper::net::Openssl;

  let ssl = Openssl::with_cert_and_key("assets/server.crt", "assets/server.key").unwrap();
  let mut server = Nickel::new();

  server.utilize(middleware! { |request|
    println!("logging request from middleware! macro: {:?}", request.origin.uri);
  });

  server.post("/a/post/request", middleware! { |request, response|
    let person = request.json_as::<Person>().unwrap();
    format!("Hello {} {}", person.firstname, person.lastname)
  });
  server.get("**", middleware!("Hello World from HTTPS"));

  server.listen_https("127.0.0.1:6767", ssl);
}
