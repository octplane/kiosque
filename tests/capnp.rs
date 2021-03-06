#![feature(test)]

extern crate log_archive;
extern crate logformat;
extern crate capnp;
extern crate rand;
extern crate chrono;
extern crate test;

use std::fs::File;
use capnp::message::Builder;
use capnp::serialize;

use rand::Rng;
use chrono::duration::Duration;
use chrono::datetime::DateTime;
use chrono::offset::utc::UTC;
use chrono::Timelike;

use logformat::schema_capnp::{logblock, logline};

pub fn random_ip() -> String {
    let mut rng = rand::thread_rng();

    let a: u32 = rng.gen_range(1, 255);
    let b: u32 = rng.gen_range(1, 255);
    let c: u32 = rng.gen_range(0, 255);
    let d: u32 = rng.gen_range(2, 255);
    format!("{}.{}.{}.{}", a, b, c, d)
}

static APACHE_FORMAT: &'static str = "%d/%b/%Y:%H:%M:%S %z";
static REFERERS: &'static [&'static str] = &["-",
                                             "http://www.casualcyclist.com",
                                             "http://bestcyclingreviews.com/top_online_shops",
                                             "http://bleater.com",
                                             "http://searchengine.com"];

static VERBS: &'static [&'static str] = &["GET", "POST", "PUT", "DELETE", "HEAD"];

static USERAGENTS: &'static [&'static str] =
    &["Mozilla/4.0 (compatible; MSIE 7.0; Windows NT 6.0)",
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_9_2) AppleWebKit/537.36 (KHTML, like Gecko) \
       Chrome/36.0.1944.0 Safari/537.36",
      "Mozilla/5.0 (Linux; U; Android 2.3.5; en-us; HTC Vision Build/GRI40) AppleWebKit/533.1 \
       (KHTML, like Gecko) Version/4.0 Mobile Safari/533.1",
      "Mozilla/5.0 (iPad; CPU OS 6_0 like Mac OS X) AppleWebKit/536.26 (KHTML, like Gecko) \
       Version/6.0 Mobile/10A5355d Safari/8536.25",
      "Mozilla/5.0 (Windows; U; Windows NT 6.1; rv:2.2) Gecko/20110201",
      "Mozilla/5.0 (Windows NT 5.1; rv:31.0) Gecko/20100101 Firefox/31.0",
      "Mozilla/5.0 (Windows; U; MSIE 9.0; WIndows NT 9.0; en-US))"];
static RESOURCES: &'static [&'static str] = &["/handle-bars",
                                              "/stems",
                                              "/wheelsets",
                                              "/forks",
                                              "/seatposts",
                                              "/saddles",
                                              "/shifters",
                                              "/Store/cart.jsp?productID="];

pub fn apache_time(start: DateTime<UTC>, us: i64) -> (DateTime<UTC>, String) {
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
    let mut times = Vec::<i64>::with_capacity(event_count as usize);

    for _ in 1..event_count + 1 {
        let _ = times.push(rng.gen_range(0, total_duration.num_seconds()) * 1000000);
    }
    times.sort();
    let o = times.into_iter()
        .map(|time| {
            let r = REFERERS[rng.gen_range(0, REFERERS.len())];
            let ua = USERAGENTS[rng.gen_range(0, USERAGENTS.len())];
            let uri = RESOURCES[rng.gen_range(0, RESOURCES.len())];
            let verb = VERBS[rng.gen_range(0, VERBS.len())];

            let prefix = match uri.find("Store") {
                Some(_) => format!("{}{}", uri, rng.gen_range(1000, 1500)),
                None => "".into(),
            };

            let (datetime, s_time) = apache_time(now, time);
            let line = format!("{} - - [{}] \"{} {}{} HTTP/1.0\" 200 {} \"{}\" \"{}\"",
                               random_ip(),
                               s_time,
                               verb,
                               uri,
                               prefix,
                               rng.gen_range(2000, 5000),
                               r,
                               ua);
            (datetime, line)
        })
        .collect();
    return o;
}


pub fn append_line(ts: DateTime<UTC>, content: &str, lline: &mut logline::Builder) {
    let second_in_micro = ts.timestamp() as u64 * 1000000;
    let us: u64 = (ts.nanosecond() / 1000) as u64;
    let timestamp_in_micro: u64 = (second_in_micro + us) as u64;

    lline.set_time(timestamp_in_micro);
    lline.set_facility("Test Facility".into());
    {
        let mut facets_builder = lline.borrow().init_facets();
        let mut kv = facets_builder.borrow().init_entries(1);

        let _ = kv.borrow().get(0).set_key("stdout");
        let _ = kv.borrow().get(0).set_value(content);
    }
}


pub fn build_log_block(file_suffix: &str, counter: u32, logblock_size: u32) {
    let mut message = Builder::new_default();
    {
        let mut block = message.init_root::<logblock::Builder>();
        let mut lines = block.borrow().init_entries(logblock_size);

        for (ix, (ts, line)) in line_generator(logblock_size).into_iter().enumerate() {
            let lines_ix: u32 = ix as u32;
            let mut cline = lines.borrow().get(lines_ix);
            append_line(ts, &line, &mut cline);
        }
    }
    let file_name = format!("{}{}.capnp", file_suffix, counter);
    let mut f = File::create(file_name.clone()).unwrap();
    let _ = serialize::write_message(&mut f, &message);
    println!("Wrote {} lines to {}.", logblock_size, file_name);
}

#[cfg(test)]
mod capnp_tests {
    use super::*;
    use log_archive::logmanager::new_from_files;
    use chrono::*;
    use std::thread;
    use rand;
    use rand::Rng;

    #[test]
    #[ignore]
    fn generate_20_files() {
        println!("Generating test files...");


        let mut threads = vec![];

        for ix in 0..8 {
            let t = thread::Builder::new()
                .name(format!("file-thread-{}", ix))
                .spawn(move || {
                    println!("{} started.", ix);
                    for x in 1..21 {
                        let index = ix * 20 + x;
                        build_log_block("data/sample", index, 500000);
                    }
                    ()
                });
            threads.push(t.ok().unwrap());
        }

        for t in threads {
            t.join().unwrap();
        }
    }

    #[test]
    fn search_things() {
        let mut rng = rand::thread_rng();
        let amount = 7000 / 128; // MEM in MB you want to use / size of log file 128MB
        let thread_count = 8;

        let files = (1..amount)
            .map(|_| {
                let ix: u32 = rng.gen_range(1, 159);

                format!("data/sample{}.capnp", ix)
            })
            .collect();
        let mut lm = new_from_files(8, files);


        bench("[HIT] GET with REGEXP", &mut lm, |lm| {
            let matches = lm.find("stdout", "GET", true);
            assert!(matches > 0, "matches: {} when if should not", matches);
        });

        bench("[HIT] GET without REGEXP", &mut lm, |lm| {
            let matches = lm.find("stdout", "GET", false);
            assert!(matches > 0, "matches: {} when if should not", matches);
        });

        bench("[HIT] http://bestcyclingreviews.com/top_online_shops with REGEXP",
              &mut lm,
              |lm| {
                  let matches = lm.find("stdout",
                                        "http://bestcyclingreviews.com/top_online_shops",
                                        true);
                  assert!(matches > 0, "matches: {} when if should not", matches);
              });

        bench("[MISS] with REGEXP", &mut lm, |lm| {
            let matches = lm.find("stdout", "missing string in data", true);
            assert!(matches == 0,
                    "We have some matches ({}) for something that shouldn't",
                    matches);
        });

        lm.shutdown();
    }

    fn bench<F>(i: &str, lm: &mut ::log_archive::logmanager::LogManager, f: F)
        where F: Fn(&mut ::log_archive::logmanager::LogManager)
    {
        let start: DateTime<UTC> = UTC::now();       // e.g. `2014-11-28T12:45:59.324310806Z`
        f(lm);
        let end: DateTime<UTC> = UTC::now();       // e.g. `2014-11-28T12:45:59.324310806Z`
        let duration = end - start;
        println!("{}: tooks {}ms.", i, duration.num_milliseconds());
    }
}
