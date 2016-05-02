use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader};
use std::thread;
use std::thread::{Thread, JoinHandle};
use std::sync::mpsc::{channel, Sender, Receiver};

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
  pub fn find(&self, field: &str, needle: &str) -> bool {
    let matching_lines = self.lines.iter().filter( |line| {
      if let Some(haystack) = line.get(field) {
        match haystack.find(needle) {
          Some(_) => true,
          _ => false
        }
      } else {
        false
      }
    });
    matching_lines.count() > 0
  }
}

pub struct LogFileThread {
  pub content: Vec<LogFile>,
}


impl LogFileThread {
  pub fn len(&self) -> usize {
    self
      .content
      .iter()
      .map( |lf| lf.len() )
      .fold(0, |acc, x| acc + x)
  }
  pub fn find(&self, field: &str, needle: &str) -> bool {
    self
      .content
      .iter()
      .filter( |lf| lf.find(field, needle))
      .count() > 0
  }
  pub fn run(&mut self, rx: Receiver<ManagerMessages>, tx: Sender<ClientMessages>) {
    println!("Thread starting.");
    loop {
      match rx.recv() {
        Ok(ManagerMessages::ReadFile(file)) =>
        {
          match read_log_block(&file) {
            Ok(lf) => {
              self.content.push(lf)
            },
            Err(e) => println!("Something went wrong while reading {}: {:?}", file,  e)
          }
        },
        Ok(ManagerMessages::FindNeedle(field, needle)) => {
          println!("needle: {} among {} files", self.find(&field, &needle), self.content.len());
        },
        Ok(ManagerMessages::Shutdown(msg)) => {
          println!("Will shutdown because {}.", msg);
          break;
        },
        Err(e) => println!("Will soon die: {}", e)
      }
    }
  }
}

#[derive(Debug)]
pub enum ManagerMessages {
  ReadFile(String),
  FindNeedle(String, String),
  Shutdown(String)
}

#[derive(Debug)]
pub enum ClientMessages {
  FoundNeedle(String, String)
}

pub struct LogManager {
  pub threads: Vec<JoinHandle<()>>,
  pub tx_chans: Vec<Sender<ManagerMessages>>,
  pub rx_chan: Receiver<ClientMessages>
}

impl LogManager {
  pub fn shutdown(self) {
    for t in self.threads {
      let _ = t.join();
    }
  }
  pub fn find(&mut self, field: &str, needle: &str) -> bool {
    for chan in &self.tx_chans {
      chan
        .send(ManagerMessages::FindNeedle(
            field.into(),
            needle.into()))
        .unwrap();
    }
    true
  }
}

pub fn LogManager(thread_count: u32, files: Vec<String>) -> LogManager {
  let mut threads: Vec<JoinHandle<()>> = Vec::new();
  let (thread_to_manager_tx, thread_to_manager_rx) = channel::<ClientMessages>();

  let chans: Vec<Sender<ManagerMessages>> = (0..thread_count).map( |ix| {
    let (manager_to_thread_tx, manager_to_thread_rx) = channel::<ManagerMessages>();
    let tmtx = thread_to_manager_tx.clone();

    let t = thread::spawn(move|| {
      let mut l = LogFileThread{content: vec![]};
      l.run(manager_to_thread_rx, tmtx);
    });
    threads.push(t);
    manager_to_thread_tx
  }).collect();


  for (ix, file) in files.iter().enumerate() {
    println!("Sending {} to thread {}", file, ix % chans.len());
    chans[ix % chans.len()]
      .send(ManagerMessages::ReadFile(file.clone()))
      .unwrap();
  }

  LogManager{
    threads: threads,
    tx_chans: chans,
    rx_chan: thread_to_manager_rx
  }
}


pub fn read_log_block(file_name: &str) -> Result<LogFile, ReadError> {
  let f = try!(File::open(file_name));
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

  println!("Read {}", file_name);
  Ok(LogFile{lines:lines})
}



