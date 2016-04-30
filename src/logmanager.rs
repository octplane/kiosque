use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader};

use capnp;
use capnp::serialize_packed;
use logformat::schema_capnp::{logblock};
// We derive `Debug` because all types should probably derive `Debug`.
#[derive(Debug)]
pub enum ReadError {
    Io(io::Error),
    Proto(capnp::Error),
}

impl From<io::Error> for ReadError {
    fn from(err: io::Error) -> ReadError {
        ReadError::Io(err)
    }
}

impl From<capnp::Error> for ReadError {
    fn from(err: capnp::Error) -> ReadError {
        ReadError::Proto(err)
    }
}

pub struct LogFile {
  pub lines: Vec<HashMap<String,String>>
}

impl LogFile {
  pub fn len(&self) -> usize {
    self
      .lines
      .iter()
      .map( |l| l.len() )
      .fold(0, |acc, x| acc + x)
  }
}

pub struct LogManager {
  pub content: Vec<LogFile>
}

impl LogManager {
  pub fn len(&self) -> usize {
    self
      .content
      .iter()
      .map( |lf| lf.len() )
      .fold(0, |acc, x| acc + x)
  }
}

pub fn read_files(files: Vec<String>) -> LogManager {
  let content = files.iter().map( |file| {
    match read_log_block(file) {
      Ok(c) => c,
      Err(e) => panic!("Error: {:?}", e)
    }
  }).collect();
  LogManager{content:content}
}


pub fn read_log_block(file_name: &str) -> Result<LogFile, ReadError> {
  let f = try!(File::open(file_name));
  println!("{}", file_name);
  let mut bufreader = BufReader::new(f); 
  let message_reader = try!(serialize_packed::read_message(
      &mut bufreader,
      ::capnp::message::ReaderOptions::new()));
  let logblock = try!(message_reader.get_root::<logblock::Reader>());
  let entries = try!(logblock.get_entries());
  
  let lines = entries.iter().map(|line_reader| {
    let t = line_reader.get_time();
    let f = line_reader.get_facility().unwrap();

    if let Ok(facets) = line_reader.get_facets() {
      if let Ok(entries) = facets.get_entries() {
        entries.iter().map(|ent| {
          (ent.get_key().ok().unwrap().to_owned(),
          ent.get_value().ok().unwrap().to_owned())
        }).collect::<HashMap<String,String>>()
      } else {
        HashMap::<String,String>::new()
      }
    } else {
        HashMap::<String,String>::new()
    }
  }).collect();

  Ok(LogFile{lines:lines})
}



