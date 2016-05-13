//! Catapult is a simple replacement for logstash written in Rust
//!
//! It aims at being a simple logshipper that read logs from its inputs, transforms them using its filters
//! and send them to its outputs.

#[macro_use]
extern crate nom;

extern crate log_archive;

extern crate docopt;

use log_archive::configuration_items;
use log_archive::config;
use docopt::Docopt;

// Write the Docopt usage string. dfrites ?
static USAGE: &'static str = "
Usage: kiosque [-c CONFIGFILE]
       kiosque (--help | -h)

Options:
    -h, --help     Show this screen.
    -c CONFIGFILE  Configuration file [default: kiosque.conf]
";

fn main() {
    // Parse argv and exit the program with an error message if it fails.
    let args = Docopt::new(USAGE)
        .and_then(|d| d.argv(std::env::args().into_iter()).parse())
        .unwrap_or_else(|e| e.exit());

    let config_file = args.get_str("-c");

    match config::read_config_file(config_file) {
      Ok(configuration) => run(configuration),
      Err(e) => panic!("Unable to parse config file at {}: {}", config_file, e)
    }
}

fn run(conf: log_archive::config::Configuration) {
  println!("{:?}", conf);
}
