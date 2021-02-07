mod dummy;
use dummy::DummyPlugin;

use crate::model::EntryPlugin;

use serde::{Deserialize, Serialize};

use std::path::PathBuf;

#[cfg(feature = "plugin-lua")]
mod luaplugin;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LoadablePlugins {
    Dummy,
    Lua(LuaConfig),
}

impl LoadablePlugins {
    pub fn load(&self) -> Box<dyn EntryPlugin> {
        match self {
            Self::Dummy => Box::new(DummyPlugin {}),
            Self::Lua(conf) => conf.load(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LuaConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(alias = "source")]
    pub file: PathBuf,
}

impl LuaConfig {
    #[cfg(feature = "plugin-lua")]
    pub fn load(&self) -> Box<dyn EntryPlugin> {
        let res = luaplugin::LuaPlugin::new(self.clone());
        let name = self.name.as_deref().unwrap_or_default();
        match res {
            Ok(plugin) => Box::new(plugin),
            Err(e) => {
                eprintln!(
                    "ERROR: Could not load Lua plugin {:?} from {:?}: {:?}",
                    name, self.file, e
                );
                Box::new(DummyPlugin {})
            }
        }
    }
    #[cfg(not(feature = "plugin-lua"))]
    pub fn load(&self) -> Box<dyn EntryPlugin> {
        let name = self.name.as_deref().unwrap_or_default();
        eprintln!("Warning: Attempted to load Lua plugin {:?} from {:?}, but Lua support has been disabled!", name, self.file);
        Box::new(DummyPlugin {})
    }
}
