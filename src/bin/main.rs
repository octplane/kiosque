extern crate log_archive;
#[macro_use]
extern crate timeit;

use log_archive::logmanager::{new_from_files};


fn main() {
    let files = (0..1000).map( |ix| 
                            format!("sample{}.capnp", ix))
      .collect();
    let mut lm = new_from_files(10, files);
    timeit!(
    {
      println!("{}", lm.find("stdout", "GET"));
    });

    lm.shutdown();
}

