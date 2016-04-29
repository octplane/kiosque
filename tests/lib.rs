extern crate log_archive;
extern crate logformat;
extern crate capnp;
extern crate rand;
extern crate chrono;

use rand::Rng;

pub fn random_ip() -> String {
  let mut rng = rand::thread_rng();

  let a: u32 = rng.gen_range(1,255);
  let b: u32 = rng.gen_range(1,255);
  let c: u32 = rng.gen_range(0,255);
  let d: u32 = rng.gen_range(2,255);
  format!("{}.{}.{}.{}", a,b,c,d)
}

use chrono::duration::Duration;
use chrono::datetime::DateTime;
use chrono::offset::utc::UTC;
use chrono::Timelike;
use std::fmt::Display;

static APACHE_FORMAT:&'static str = "%d/%b/%Y:%H:%M:%S %z";
static REFERERS: &'static [&'static str] = &["-","http://www.casualcyclist.com","http://bestcyclingreviews.com/top_online_shops","http://bleater.com","http://searchengine.com"];
static USERAGENTS: &'static [&'static str] = &["Mozilla/4.0 (compatible; MSIE 7.0; Windows NT 6.0)","Mozilla/5.0 (Macintosh; Intel Mac OS X 10_9_2) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/36.0.1944.0 Safari/537.36","Mozilla/5.0 (Linux; U; Android 2.3.5; en-us; HTC Vision Build/GRI40) AppleWebKit/533.1 (KHTML, like Gecko) Version/4.0 Mobile Safari/533.1","Mozilla/5.0 (iPad; CPU OS 6_0 like Mac OS X) AppleWebKit/536.26 (KHTML, like Gecko) Version/6.0 Mobile/10A5355d Safari/8536.25","Mozilla/5.0 (Windows; U; Windows NT 6.1; rv:2.2) Gecko/20110201","Mozilla/5.0 (Windows NT 5.1; rv:31.0) Gecko/20100101 Firefox/31.0","Mozilla/5.0 (Windows; U; MSIE 9.0; WIndows NT 9.0; en-US))"];
static RESOURCES: &'static [&'static str] = &["/handle-bars","/stems","/wheelsets","/forks","/seatposts","/saddles","/shifters","/Store/cart.jsp?productID="];

pub fn apache_time(start: DateTime<UTC>, us: i64 ) -> (DateTime<UTC>, String) {
  let d = Duration::microseconds(us);
  if let Some(r) = start.checked_add(d) {
    (r, r.format(APACHE_FORMAT).to_string())
  } else {
    (start, start.format(APACHE_FORMAT).to_string())
  }
}

pub fn line_generator(event_count: u32) -> Vec<(DateTime<UTC>, String)> {
  let now = UTC::now();
  let mut rng = rand::thread_rng();
  let total_duration = Duration::seconds(rng.gen_range(10, 3600 * 3));
  println!("Will generate {} events across {} seconds.", event_count, total_duration.num_seconds());
  let mut times = Vec::<i64>::with_capacity(event_count as usize);

  for x in 1..event_count {
    times.push(rng.gen_range(0,total_duration.num_seconds()) * 1000000 );
  }
  times.sort();
  times.into_iter().map( |time| {
    let r = REFERERS[rng.gen_range(0, REFERERS.len())];
    let ua = USERAGENTS[rng.gen_range(0, USERAGENTS.len())];
    let mut uri = RESOURCES[rng.gen_range(0, RESOURCES.len())]; 

    if let Some(_) = uri.find("Store") {
      let uri = &format!("{}{}", uri, rng.gen_range(1000, 1500));
    }

    let (datetime, s_time) = apache_time(now, time);
    let line = format!("{} - - [{}] \"GET {} HTTP/1.0\" 200 {} \"{}\" \"{}\"",
                       random_ip(), s_time , uri, rng.gen_range(2000, 5000),
                       r, ua);
    (datetime, line)
  }).collect()
}



#[cfg(test)]
mod tests {
  use super::*;
  use logformat::schema_capnp::{logblock};
  use capnp::message::Builder;
  use capnp::serialize;
  use chrono::Timelike;

  use std::collections::HashMap;


  #[test]
  fn it_works() {

    let mut message = Builder::new_default();
    {
      let logblock_sz = 500;
      let mut block = message.init_root::<logblock::Builder>();
      let mut lines = block.borrow().init_entries(logblock_sz);

      for (ix, (ts, line)) in line_generator(logblock_sz).into_iter().enumerate() {
        let lines_ix: u32 = ix as u32;
        let second_in_micro = ts.timestamp() as u64 * 1000000;
        let us: u64 = (ts.nanosecond() / 1000) as u64; 
        let timestamp_in_micro: u64  = ( second_in_micro + us) as u64;
        let sline: &str = &line;

        let mut cline = lines.borrow().get(lines_ix);
        cline.set_time(timestamp_in_micro);
        cline.set_facility("Test Facility".into());
        {
          let mut facets_builder = cline.borrow().init_facets();
          let mut kv = facets_builder.borrow().init_entries(1);

          kv.borrow().get(0).set_key("stdout");
          kv.borrow().get(0).set_value(sline);
        }
      }
    }
    let mut vec = Vec::new();
    serialize::write_message(&mut vec, &message);
    assert!(vec.len() == 12, "vec len: {}", vec.len());
  }
}
