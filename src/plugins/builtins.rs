mod freedesktop;
mod rawpath;

use freedesktop::FreedesktopPlugin;
use rawpath::RawPathPlugin;

use crate::model::EntryPlugin;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinPlugins {
    #[serde(rename = "xdg")]
    Freedesktop,
    #[serde(rename = "path")]
    RawPath,
}

impl BuiltinPlugins {
    pub fn load(&self) -> Box<dyn EntryPlugin> {
        match self {
            BuiltinPlugins::RawPath => Box::new(RawPathPlugin::new()),
            BuiltinPlugins::Freedesktop => Box::new(FreedesktopPlugin::new()),
        }
    }
}
