extern crate log_archive;

use log_archive::logmanager::{new_from_files};


fn main() {
    let files = (0..3).map( |ix| 
                            format!("sample{}.capnp", ix))
      .collect();
    let mut lm = new_from_files(4, files);
    println!("{}", lm.find("stdout", "GET"));
    lm.shutdown();


}

