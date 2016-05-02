extern crate log_archive;

use log_archive::logmanager::{LogManager};


fn main() {
    let files = (0..3).map( |ix| 
                            format!("sample{}.capnp", ix))
      .collect();
    let mut lm = LogManager(4, files);
    println!("{}", lm.find("stdout", "GET"));
    //println!("Len is {}", lm.len());
    lm.shutdown();


}

