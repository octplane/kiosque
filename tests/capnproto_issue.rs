extern crate log_archive;

#[cfg(test)]
mod tests {
  use log_archive::logmanager::read_log_block;


 #[test]
 fn broken_file() {
   let file = "broken/sample148.capnp";
   let _ = read_log_block(file);
 }
}
