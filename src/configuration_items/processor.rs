use std::collections::HashMap;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;
use std::sync::mpsc::sync_channel;
use std::thread::JoinHandle;

pub trait ConfigurableFilter {
    fn human_name(&self) -> &str;
    fn mandatory_fields(&self) -> Vec<&str> {
        vec![]
    }

    fn requires_fields(&self,
                       optional_config: &Option<HashMap<String, String>>,
                       required_fields: Vec<&str>) {
        let mut missing_fields = Vec::new();
        match optional_config {
            &Some(ref config) => {
                for required in required_fields {
                    if !config.contains_key(required) {
                        missing_fields.push(required);
                    }
                }
            }
            &None => missing_fields.extend(&required_fields),
        }

        if missing_fields.len() > 0 {
            panic!("Missing fields for \"{}\": {:?}",
                   self.human_name(),
                   missing_fields);
        }
    }
}

pub trait InputProcessor: ConfigurableFilter {
    #[allow(unused_variables)]
    fn start(&self, config: &Option<HashMap<String, String>>) -> Receiver<String> {
        panic!("Not implemented");
    }

    #[allow(unused_variables)]
    fn handle_func(tx: SyncSender<String>, config: Option<HashMap<String, String>>) {
        panic!("Not implemented");
    }

    fn invoke(&self,
              config: &Option<HashMap<String, String>>,
              handle_func: fn(tx: SyncSender<String>,
                              config: Option<HashMap<String, String>>))
              -> Receiver<String> {
        let (tx, rx) = sync_channel(10000);
        let conf = config.clone();

        let run_loop = thread::Builder::new().name("run_loop".to_string()).spawn(move || {
            handle_func(tx, conf);
        });

        match run_loop {
            Ok(_) => {
                println!("Started Thread for {}", self.human_name());
                rx
            }
            Err(e) => panic!("Unable to spawn {} input thread: {}", self.human_name(), e),
        }
    }
}

pub trait OutputProcessor: ConfigurableFilter {
    fn start(&self,
             _rx: Receiver<String>,
             _config: &Option<HashMap<String, String>>)
             -> Result<JoinHandle<()>, String> {
        panic!("Not implemented");
    }

    #[allow(unused_variables)]
    fn handle_func(rx: Receiver<String>, config: Option<HashMap<String, String>>) {
        panic!("Not implemented");
    }

    fn invoke(&self,
              rx: Receiver<String>,
              config: &Option<HashMap<String, String>>,
              handle_func: fn(rx: Receiver<String>,
                              config: Option<HashMap<String, String>>))
              -> Result<JoinHandle<()>, String> {
        let conf = config.clone();

        let run_loop = thread::Builder::new().name("run_loop".to_string()).spawn(move || {
            handle_func(rx, conf);
        });

        match run_loop {
            Ok(jh) => Ok(jh),
            Err(e) => Err(format!("Unable to spawn {} output thread: {}", self.human_name(), e)),
        }
    }
}
