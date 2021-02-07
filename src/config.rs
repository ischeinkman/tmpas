use crate::model::ListEntry;
use crate::plugins::{BuiltinPlugins, LoadablePlugins};

use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt;

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

    #[serde(default, alias = "ui")]
    pub interfaces: HashMap<UiTag, UiConfig>,
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

    pub fn is_interface_enabled(&self, tag: UiTag) -> bool {
        if cfg!(not(feature = "iced-ui")) && tag == UiTag::Iced {
            return false;
        }
        if cfg!(not(feature = "smithay-ui")) && tag == UiTag::Smithay {
            return false;
        }

        if cfg!(not(feature="crossterm-ui")) && tag == UiTag::Crossterm {
            return false;
        }

        self.interfaces
            .get(&tag)
            .map(|conf| conf.enable)
            .unwrap_or(true)
    }
    pub fn default_interface(&self) -> UiTag {
        let mut defaultable = Vec::new();
        let mut available = Vec::new();
        for tag in UiTag::all().iter() {
            if !self.is_interface_enabled(*tag) {
                continue;
            }
            let conf = self.interfaces.get(tag).cloned().unwrap_or_default();
            match conf.default {
                Some(true) => {
                    return *tag;
                }
                None => {
                    defaultable.push(tag);
                }
                Some(false) => {
                    available.push(tag);
                }
            }
        }
        defaultable.sort();
        available.sort();
        defaultable
            .first()
            .or_else(|| available.first())
            .map(|tag| **tag)
            .expect("All interfaces are currently disabled?")
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(default)]
pub struct UiConfig {
    pub enable: bool,
    pub default: Option<bool>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            enable: true,
            default: None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum UiTag {
    Iced,
    Smithay,
    Crossterm,
}

impl UiTag {
    pub const fn all() -> &'static [UiTag] {
        &[UiTag::Iced, UiTag::Crossterm, UiTag::Smithay]
    }
}

impl PartialOrd for UiTag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for UiTag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (UiTag::Iced, UiTag::Iced)
            | (UiTag::Smithay, UiTag::Smithay)
            | (UiTag::Crossterm, UiTag::Crossterm) => std::cmp::Ordering::Equal,
            (UiTag::Iced, _) => std::cmp::Ordering::Greater,
            (_, UiTag::Iced) => std::cmp::Ordering::Less,
            (UiTag::Smithay, UiTag::Crossterm) => std::cmp::Ordering::Less,
            (UiTag::Crossterm, UiTag::Smithay) => std::cmp::Ordering::Less,
        }
    }
}

use serde::{Deserializer, Serializer};

impl serde::Serialize for UiTag {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let tag = match *self {
            UiTag::Crossterm => "terminal",
            UiTag::Smithay => "graphical",
            UiTag::Iced => "graphical-iced",
        };
        serializer.serialize_str(tag)
    }
}

impl<'a> serde::Deserialize<'a> for UiTag {
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(UiTagVisitor {})
    }
}

struct UiTagVisitor {}

impl<'a> serde::de::Visitor<'a> for UiTagVisitor {
    type Value = UiTag;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid user interface tag")
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            "terminal" | "crossterm" => Ok(UiTag::Crossterm),
            "graphical-iced" | "iced" => Ok(UiTag::Iced),
            "graphical" | "smithay" => Ok(UiTag::Smithay),
            other => Err(E::unknown_variant(other, &["terminal", "graphical"])),
        }
    }
}
