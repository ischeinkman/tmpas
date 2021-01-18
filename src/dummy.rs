use crate::config::Config;
use crate::model::{EntryPlugin, ListEntry};

pub struct DummyPlugin {}

impl EntryPlugin for DummyPlugin {
    fn name(&self) -> String {
        "Dummy".to_owned()
    }
    fn start(&mut self, _: &Config) {}
    fn next(&mut self) -> Option<ListEntry> {
        None
    }
}
