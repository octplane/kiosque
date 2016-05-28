use std::fs::File;
use std::fmt;
use std::io::{self, Read};
use std::thread;
use std::borrow::Borrow;
use std::thread::JoinHandle;
use std::sync::mpsc::{channel, Sender, Receiver};
use itertools::Itertools;

use capnp;
use capnp::serialize_packed;
use logformat::schema_capnp::{logblock, logline};

use regex::Regex;

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
    pub content: Vec<u8>,
}

fn find_field<'a>(entry: &'a logline::Reader, field: &str) -> Option<&'a str> {
    match entry.get_facets() {
        Ok(facets) => {
            match facets.get_entries() {
                Ok(entries) => {
                    for kv in entries.iter() {
                        let k = kv.get_key().ok().unwrap();
                        if k == field {
                            return Some(kv.get_value().ok().unwrap());
                        }
                    }
                }
                _ => {
                    return None;
                }
            }
        }
        _ => {
            return None;
        }
    }
    None
}

pub type LogSearchResult = u64;

impl LogFile {
    pub fn gen_find<F>(&self, field: &str, needle: &str, testf: F) -> LogSearchResult
        where F: Fn(&str, &str) -> bool
    {
        let res = match serialize_packed::read_message(&mut self.content.borrow(),
                                                       ::capnp::message::ReaderOptions::new()) {
            Ok(message_reader) => {
                match message_reader.get_root::<logblock::Reader>() {
                    Ok(logblock) => {
                        match logblock.get_entries() {
                            Ok(entries) => {
                                entries.iter().fold(0, |acc, line_reader| {
                                    match find_field(&line_reader, field) {
                                        Some(content) => {
                                            if testf(needle, content) {
                                                acc + 1
                                            } else {
                                                acc
                                            }
                                        }
                                        _ => acc,
                                    }
                                })
                            }
                            _ => 0,
                        }
                    }
                    _ => 0,
                }
            }
            _ => 0,
        };
        println!("Looking for {} in field {}: Got {:?} matches",
                 needle,
                 field,
                 res);
        res
    }

    pub fn find(&self, field: &str, needle: &str) -> u64 {
        self.gen_find(field, needle, |needle, haystack| {
            match haystack.find(needle) {
                Some(_) => true,
                _ => false,
            }
        })
    }
    pub fn rfind(&self, field: &str, needle: &str) -> u64 {
        let re = Regex::new(needle).unwrap();
        self.gen_find(field, needle, |_, haystack| re.is_match(haystack))
    }
}

pub struct LogFileThread {
    pub name: String,
    pub content: Vec<LogFile>,
}



impl LogFileThread {
    pub fn find(&self, field: &str, needle: &str) -> LogSearchResult {
        self.content
            .iter()
            .fold(0, |acc, lf| acc + lf.find(field, needle))
    }
    pub fn rfind(&self, field: &str, needle: &str) -> LogSearchResult {
        self.content
            .iter()
            .fold(0, |acc, lf| acc + lf.rfind(field, needle))
    }
    pub fn run(&mut self, rx: Receiver<ManagerMessages>, tx: Sender<ClientMessages>) {
        println!("{}: Thread starting.", self.name);
        loop {
            match rx.recv() {
                Ok(ManagerMessages::ReadFiles(files)) => {
                    println!("Reading {:?}", files);
                    for file in &files {
                        match read_log_block(&file) {
                            Ok(lf) => {
                                println!("{}: Read file {}", self.name, file);
                                self.content.push(lf);
                            }
                            Err(e) => {
                                println!("{}: Something went wrong while reading {}: {:?}",
                                         self.name,
                                         file,
                                         e)
                            }
                        }
                    }
                    tx.send(ClientMessages::ReadFiles(files));
                }
                Ok(ManagerMessages::FindNeedle(field, needle, r_based)) => {
                    let found = if r_based {
                        self.rfind(&field, &needle)
                    } else {
                        self.find(&field, &needle)
                    };

                    let _ = if found > 0 {
                        println!("FOUND IS {}", found);

                        tx.send(ClientMessages::FoundNeedle(self.name.clone(), field, needle))
                    } else {
                        println!("FOUND IS 0 ({})", needle);
                        tx.send(ClientMessages::NotFound(self.name.clone(), field, needle))
                    };
                }
                Ok(ManagerMessages::Shutdown(msg)) => {
                    println!("{}: Will shutdown because {}.", self.name, msg);
                    break;
                }
                Err(e) => println!("{}: Error while recv(): {}", self.name, e),
            }
        }
        println!("{}: Thread stopping", self.name);
    }
}

#[derive(Debug)]
pub enum ManagerMessages {
    ReadFiles(Vec<String>),
    FindNeedle(String, String, bool),
    Shutdown(String),
}

#[derive(Debug)]
pub enum ClientMessages {
    ReadFiles(Vec<String>),
    FoundNeedle(String, String, String),
    NotFound(String, String, String),
}

pub struct LogManager {
    pub threads: Vec<JoinHandle<()>>,
    pub tx_chans: Vec<Sender<ManagerMessages>>,
    pub rx_chans: Vec<Receiver<ClientMessages>>,
}

impl LogManager {
    pub fn shutdown(self) {
        for chan in &self.tx_chans {
            chan.send(ManagerMessages::Shutdown("shutdown called".into()))
                .unwrap();
        }
        for t in self.threads {
            let _ = t.join();
        }
        println!("Shutdowning.");
    }
    pub fn find(&mut self, field: &str, needle: &str, re_based: bool) -> bool {
        for chan in &self.tx_chans {
            chan.send(ManagerMessages::FindNeedle(field.into(), needle.into(), re_based))
                .unwrap();
        }
        let mut count = 0;
        for c in &self.rx_chans {
            match c.recv() {
                Ok(ClientMessages::FoundNeedle(_, _, _)) => count = count + 1,
                Ok(ClientMessages::NotFound(t, _, _)) => println!("Miss from {}", t),
                Ok(ClientMessages::ReadFiles(fs)) => {
                    panic!("Thread re-read files {:?}, this is not normal", fs)
                }
                Err(e) => println!("Something went wront on the pipe: {}", e),
            }
        }
        println!("Found is {}", count);
        count == self.rx_chans.len()
    }
}

pub fn new_from_files(thread_count: usize, files: Vec<String>) -> LogManager {

    let t_count = if files.len() > thread_count {
        thread_count
    } else {
        files.len()
    };

    let data = (0..t_count).map(|ix| {
        let (manager_to_thread_tx, manager_to_thread_rx) = channel::<ManagerMessages>();
        let (thread_to_manager_tx, thread_to_manager_rx) = channel::<ClientMessages>();

        let t = thread::Builder::new()
            .name(format!("file-thread-{}", ix))
            .spawn(move || {
                let mut l = LogFileThread {
                    name: format!("file-thread-{}", ix),
                    content: vec![],
                };
                l.run(manager_to_thread_rx, thread_to_manager_tx);
            })
            .ok()
            .unwrap();
        (t, manager_to_thread_tx, thread_to_manager_rx)
    });

    let mut tx_chans = vec![];
    let mut rx_chans = vec![];
    let mut threads = vec![];

    for (t, tx, rx) in data {
        threads.push(t);
        tx_chans.push(tx);
        rx_chans.push(rx);
    }


    // sort by modulo, and group by to initialize the Readers properly
    for (key, group) in files.iter()
        .enumerate()
        .sort_by(|a, b| (a.0 % t_count).cmp(&(b.0 % t_count)))
        .iter()
        .group_by(|elt| elt.0 % t_count) {
        tx_chans[key]
            .send(ManagerMessages::ReadFiles(group.iter()
                .map(|ix_file| ix_file.1.clone())
                .collect()))
            .unwrap();
    }

    let mut ready = 0;
    for rx in &mut rx_chans {
        match rx.recv() {
            Ok(ClientMessages::ReadFiles(_)) => ready = ready + 1,
            Ok(m) => panic!("Unexpected thread message: {:?}", m),
            Err(e) => println!("Something went wront on the pipe: {}", e),
        }
    }
    if ready != t_count {
        panic!("Unable to read all files");
    } else {
        println!("Read {} files in {} threads", files.len(), t_count);
    }

    LogManager {
        threads: threads,
        tx_chans: tx_chans,
        rx_chans: rx_chans,
    }
}


pub fn read_log_block(file_name: &str) -> Result<LogFile, ReadError> {
    let mut f = try!(File::open(file_name));
    let mut buffer = Vec::new();

    try!(f.read_to_end(&mut buffer));
    Ok(LogFile { content: buffer })
}
