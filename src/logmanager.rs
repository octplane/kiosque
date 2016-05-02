use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader};
use std::thread;
use std::thread::{JoinHandle};
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
  pub name: String,
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
    println!("{}: Thread starting.", self.name);
    loop {
      match rx.recv() {
        Ok(ManagerMessages::ReadFile(file)) =>
        {
          match read_log_block(&file) {
            Ok(lf) => {
              println!("{}: Read file {}", self.name, file);
              self.content.push(lf)
            },
            Err(e) => println!("{}: Something went wrong while reading {}: {:?}", self.name, file,  e)
          }
        },
        Ok(ManagerMessages::FindNeedle(field, needle)) => {
          let found = self.find(&field, &needle);
          let _ = if found {
            tx.send(ClientMessages::FoundNeedle(self.name.clone(), field, needle))
          } else {
            tx.send(ClientMessages::NotFound(self.name.clone(), field, needle))
          };
        },
        Ok(ManagerMessages::Shutdown(msg)) => {
          println!("{}: Will shutdown because {}.", self.name, msg);
          break;
        },
        Err(e) => println!("{}: Error while recv(): {}", self.name, e)
      }
    }
  println!("{}: Thread stopping", self.name);
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
  FoundNeedle(String, String, String),
  NotFound(String, String, String),
}

pub struct LogManager {
  pub threads: Vec<JoinHandle<()>>,
  pub tx_chans: Vec<Sender<ManagerMessages>>,
  pub rx_chans: Vec<Receiver<ClientMessages>>
}

impl LogManager {
  pub fn shutdown(self) {
    for chan in &self.tx_chans {
      chan
        .send(ManagerMessages::Shutdown("shutdown called".into())).unwrap();
    }
    for t in self.threads {
      let _ = t.join();
    }
    println!("Shutdowning.");
  }
  pub fn find(&mut self, field: &str, needle: &str) -> bool {
    for chan in &self.tx_chans {
      chan
        .send(ManagerMessages::FindNeedle(
            field.into(),
            needle.into()))
        .unwrap();
    }
    let mut count = 0;
    for c in &self.rx_chans {
      match c.recv() {
        Ok(ClientMessages::FoundNeedle(_, _, _)) => count = count + 1,
        Ok(ClientMessages::NotFound(t, _, _)) => println!("Miss from {}",t),
        Err(e) => println!("Something went wront on the pipe: {}", e)
      }
    }
    println!("Found is {}", count);
    count == self.rx_chans.len()
  }
}

pub fn new_from_files(thread_count: u32, files: Vec<String>) -> LogManager {

  let data = (0..thread_count).map( |ix| {
    let (manager_to_thread_tx, manager_to_thread_rx) = channel::<ManagerMessages>();
    let (thread_to_manager_tx, thread_to_manager_rx) = channel::<ClientMessages>();

    let t =  thread::Builder::new().name(format!("file-thread-{}", ix)).spawn(move|| {
      let mut l = LogFileThread {
        name: format!("file-thread-{}", ix),
        content: vec![]};
      l.run(manager_to_thread_rx, thread_to_manager_tx);
    }).ok().unwrap();
    (t, manager_to_thread_tx, thread_to_manager_rx)
  });

  let mut tx_chans = vec![];
  let mut rx_chans = vec![]; 
  let mut threads = vec![];

  for (t,tx,rx) in data {
    threads.push(t);
    tx_chans.push(tx);
    rx_chans.push(rx);
  }

  for (ix, file) in files.iter().enumerate() {
    let t_index = ix % tx_chans.len();
    tx_chans[t_index]
      .send(ManagerMessages::ReadFile(file.clone()))
      .unwrap();
  }

  LogManager{
    threads: threads,
    tx_chans: tx_chans,
    rx_chans: rx_chans
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
    // let t = line_reader.get_time();
    // let f = line_reader.get_facility().unwrap();

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



