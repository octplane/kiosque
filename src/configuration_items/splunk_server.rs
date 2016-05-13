use configuration_items::processor::{InputProcessor, ConfigurableFilter};


pub struct Splunk_Server {
  name: String,
}

impl Splunk_Server {
  pub fn new(name: String) -> Splunk_Server {
    Splunk_Server{ name: name }
  }
}

impl ConfigurableFilter for Splunk_Server {
  fn human_name(&self) -> &str {
    self.name.as_ref()
  }

  fn mandatory_fields(&self) -> Vec<&str> {
    vec![]
  }
}
