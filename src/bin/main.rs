#![feature(alloc_system)]
extern crate alloc_system;

extern crate log_archive;
extern crate hprof;

use log_archive::logmanager::{new_from_files};


fn main() {


  let files = (0..20)
    .map( |ix| format!("data/sample{}.capnp", ix))
    .collect();
  let mut lm = new_from_files(8, files);
  for _ in 1..10
  {
    let d = hprof::enter("Simple search Miss");
    println!("{}", lm.find("stdout", "internet", false));
  }

  for _ in 1..10
  {
    let d = hprof::enter("Regex search HIT");
    println!("{}", lm.find("stdout", "GET", true));
  }
  for _ in 1..10
  {
    let d = hprof::enter("Simple search HIT");
    println!("{}", lm.find("stdout", "GET", false));
  }
  for _ in 1..10
  {
    let d = hprof::enter("Regex search Miss");
    println!("{}", lm.find("stdout", "internet", true));
  }
  lm.shutdown();
  hprof::profiler().print_timing();  
}

