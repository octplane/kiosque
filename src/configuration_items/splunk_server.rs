use configuration_items::processor::{InputProcessor, ConfigurableFilter};


pub struct SplunkServer {
    name: String,
}

impl SplunkServer {
    pub fn new(name: String) -> SplunkServer {
        SplunkServer { name: name }
    }
}

impl ConfigurableFilter for SplunkServer {
    fn human_name(&self) -> &str {
        self.name.as_ref()
    }

    fn mandatory_fields(&self) -> Vec<&str> {
        vec![]
    }
}
