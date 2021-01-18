use crate::model::ListEntry;
use crate::plugins::{BuiltinPlugins, LoadablePlugins};

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub terminal: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default, rename = "plugins")]
    pub builtin_plugins: Vec<BuiltinPlugins>,

    #[serde(default, rename = "plugin")]
    pub loaded_plugins: Vec<LoadablePlugins>,
}

impl Config {
    pub fn make_terminal_command(&self, entry: &ListEntry) -> String {
        let binary = entry.exec_name().unwrap();
        let flags = entry
            .exec_command
            .iter()
            .skip(1)
            .fold(String::new(), |acc, cur| format!("{} {}", acc, cur));
        let command = format!("{} {}", binary, flags);
        let subs = [
            ("$DISPLAY_NAME", entry.name()),
            ("$BINARY", binary),
            ("$FLAGS", &flags),
            ("$COMMAND", &command),
        ];
        let mut raw = self.terminal.as_deref().unwrap_or("$COMMAND").to_owned();
        for (k, v) in &subs {
            raw = raw.replace(k, v);
        }
        raw
    }
}
