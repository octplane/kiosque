extern crate log_archive;

use log_archive::logmanager::{read_files};


fn main() {
    let files = (0..3).map( |ix| format!("sample{}.capnp", ix)).collect();
    let lm = read_files(files);

    println!("Len is {}", lm.len());


}

