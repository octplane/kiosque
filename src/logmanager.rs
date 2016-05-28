use std::fs::File;
use std::fmt;
use std::io::{self, Read};
use std::thread;
use std::borrow::Borrow;
use std::thread::JoinHandle;
use std::sync::mpsc::{channel, Sender, Receiver};
use itertools::Itertools;
use chrono::duration::Duration;
use chrono::datetime::DateTime;
use chrono::offset::utc::UTC;
use chrono::Timelike;


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

#[derive(Debug)]
pub struct LogFileStats {
    pub fname: String,
    pub size_bytes: usize,
    pub line_count: u32,
}

impl LogFileStats {
    pub fn stats(&self) {
        println!("{}: {} lines for {}",
                 self.fname,
                 self.line_count,
                 byte_to_human(self.size_bytes));
    }
}

pub struct LogFile {
    pub fname: String,
    pub content: Vec<u8>,
}


impl LogFile {
    pub fn get_stats(&self) -> LogFileStats {
        let line_count =
            match serialize_packed::read_message(&mut self.content.borrow(),
                                                 ::capnp::message::ReaderOptions::new()) {
                Ok(message_reader) => {
                    match message_reader.get_root::<logblock::Reader>() {
                        Ok(logblock) => logblock.get_entries().unwrap().len(),
                        _ => 0,
                    }
                }
                _ => 0,
            };
        LogFileStats {
            fname: self.fname.clone(),
            size_bytes: self.content.len(),
            line_count: line_count,
        }
    }

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
        let mut files_to_read = vec![];
        loop {
            match rx.recv() {
                Ok(ManagerMessages::QueueFile(file)) => files_to_read.push(file),
                Ok(ManagerMessages::ReadFiles) => {
                    println!("Reading {:?}", files_to_read);
                    for file in files_to_read.iter() {
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
                    let stats = self.content
                        .iter()
                        .map(|f| f.get_stats())
                        .collect::<Vec<LogFileStats>>();
                    tx.send(ClientMessages::ReadFiles(stats));

                    files_to_read.clear();
                }
                Ok(ManagerMessages::FindNeedle(field, needle, r_based)) => {
                    let found = if r_based {
                        self.rfind(&field, &needle)
                    } else {
                        self.find(&field, &needle)
                    };

                    let _ = if found > 0 {
                        tx.send(ClientMessages::FoundNeedle(self.name.clone(),
                                                            field,
                                                            needle,
                                                            found))
                    } else {
                        tx.send(ClientMessages::NotFound(self.name.clone(), field, needle))
                    };
                }
                Ok(ManagerMessages::Shutdown(msg)) => {
                    break;
                }
                Err(e) => println!("{}: Error while recv(): {}", self.name, e),
            }
        }
        println!("{}: Thread stopping", self.name);
    }
}

pub type LogSearchResult = u64;

#[derive(Debug)]
pub enum ManagerMessages {
    QueueFile(String),
    ReadFiles,
    FindNeedle(String, String, bool),
    Shutdown(String),
}

#[derive(Debug)]
pub enum ClientMessages {
    ReadFiles(Vec<LogFileStats>),
    FoundNeedle(String, String, String, LogSearchResult),
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
    pub fn find(&mut self, field: &str, needle: &str, re_based: bool) -> LogSearchResult {
        for chan in &self.tx_chans {
            chan.send(ManagerMessages::FindNeedle(field.into(), needle.into(), re_based))
                .unwrap();
        }
        let mut count = 0;
        for c in &self.rx_chans {
            match c.recv() {
                Ok(ClientMessages::FoundNeedle(_, _, _, c)) => count = count + c,
                Ok(ClientMessages::NotFound(t, _, _)) => {}
                Ok(ClientMessages::ReadFiles(fs)) => {
                    panic!("Thread re-read files {:?}, this is not normal", fs)
                }
                Err(e) => println!("Something went wront on the pipe: {}", e),
            }
        }
        println!("Found is {}", count);
        count
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


    let start: DateTime<UTC> = UTC::now();       // e.g. `2014-11-28T12:45:59.324310806Z`

    for (ix, file) in files.iter().enumerate() {
        tx_chans[ix % t_count]
            .send(ManagerMessages::QueueFile(file.clone()))
            .unwrap();
    }

    for chan in &mut tx_chans {
        chan.send(ManagerMessages::ReadFiles).unwrap()
    }


    let mut ready = 0;
    let mut content_stats = vec![];
    for rx in &mut rx_chans {
        match rx.recv() {
            Ok(ClientMessages::ReadFiles(stats)) => {
                content_stats.push(stats);
                ready = ready + 1
            }
            Ok(m) => panic!("Unexpected thread message: {:?}", m),
            Err(e) => println!("Something went wront on the pipe: {}", e),
        }
    }
    let end: DateTime<UTC> = UTC::now();       // e.g. `2014-11-28T12:45:59.324310806Z`
    let duration = end - start;
    println!("Tooks {}ms to read files", duration.num_milliseconds());

    if ready != t_count {
        panic!("Unable to read all files");
    } else {
        let s = compute_stats(content_stats);
        s.stats();
    }

    LogManager {
        threads: threads,
        tx_chans: tx_chans,
        rx_chans: rx_chans,
    }
}
pub fn byte_to_human(byte: usize) -> String {
    if byte > 1024 {
        if byte > 1024 * 1024 {
            format!("{} MB", byte / (1024 * 1024))
        } else {
            format!("{} kb", byte / 1024)
        }
    } else {
        format!("{} bytes", byte)
    }

}
pub fn compute_stats(stats: Vec<Vec<LogFileStats>>) -> LogFileStats {
    let f_count = stats.iter().fold(0, |acc, s| acc + s.len());
    let l_count = stats.iter().fold(0, |acc, s| {
        acc + s.iter().fold(0, |acc, lfs| acc + lfs.line_count)
    });
    let b_count = stats.iter().fold(0, |acc, s| {
        acc + s.iter().fold(0, |acc, lfs| acc + lfs.size_bytes)
    });
    LogFileStats {
        fname: format!("{} files", f_count),
        line_count: l_count,
        size_bytes: b_count,
    }
}


pub fn read_log_block(file_name: &str) -> Result<LogFile, ReadError> {
    let mut f = try!(File::open(file_name));
    let mut buffer = Vec::new();

    try!(f.read_to_end(&mut buffer));
    Ok(LogFile {
        fname: file_name.into(),
        content: buffer,
    })
}
