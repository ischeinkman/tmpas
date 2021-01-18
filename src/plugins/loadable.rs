mod dummy;
use dummy::DummyPlugin;

use crate::model::EntryPlugin;

use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LoadablePlugins {
    Dummy,
}

impl LoadablePlugins {
    pub fn load(&self) -> Box<dyn EntryPlugin> {
        match self {
            Self::Dummy => Box::new(DummyPlugin {}),
        }
    }
}
