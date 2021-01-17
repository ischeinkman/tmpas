mod freedesktop;
use freedesktop::FreedesktopPlugin;

mod rawpath;
use rawpath::RawPathPlugin;

mod utils;

mod model;
use model::{Config, EntryPlugin, ListEntry};

use std::borrow::Cow;
use std::collections::BTreeMap;

fn main() {
    let config = Config {
        list_size: 10,
        language: Some("en".to_owned()),
        terminal_runner: "alacritty --title $DISPLAY_NAME --command $COMMAND".to_owned(),
    };

    let xdgp = FreedesktopPlugin::new();
    let ptp = RawPathPlugin::new();

    let mut plugins = PluginsGroup::new();
    plugins.push(xdgp);
    plugins.push(ptp);
    plugins.start(&config);

    let mut entries: BTreeMap<String, ListEntry> = BTreeMap::new();
    let mut idx = 0;
    for mut ent in plugins {
        if let Some(cur_ent) = entries.get_mut(ent.exec_name().unwrap()) {
            let should_replace = (ent.exec_command.len() == 1 && cur_ent.exec_command.len() != 1)
                || (ent.display_name.is_some() && cur_ent.display_name.is_none());
            if should_replace {
                for child in cur_ent.children.drain(..) {
                    ent.children.push(child);
                }
                std::mem::swap(cur_ent, &mut ent);
            }
            let should_push = ent.exec_command != cur_ent.exec_command
                && (ent.exec_command.len() != 1 || cur_ent.exec_command.len() != 1);
            if should_push {
                cur_ent.children.push(ent);
                idx += 1;
            }
        } else {
            entries.insert(ent.exec_name().unwrap().to_owned(), ent);
            idx += 1;
        }
        if idx == config.list_size {
            println!("====== =================== ======");
            println!("====== STARTING LIST PRINT ======");
            println!("====== =================== ======");
            for ent in entries.values() {
                print_recursive(ent, 0);
            }
            println!("====== =================== ======");
            println!("====== StOPPING LIST PRINT ======");
            println!("====== =================== ======");
            idx = 0;
        }
    }
}

fn print_recursive(entry: &ListEntry, level: usize) {
    let prefix: Cow<'static, str> = match level {
        0 => "".into(),
        1 => "| -".into(),
        level => format!("|- {}", " --".repeat(level)).into(),
    };
    println!("{}Name: {}", prefix, entry.name());
    println!("{}Key: {}", prefix, entry.exec_name().unwrap_or_default());
    println!("{}Command: {:?}", prefix, entry.exec_command);
    println!("{}Children:", prefix);
    for child in &entry.children {
        print_recursive(child, level + 1);
    }
    println!("\n");
}

#[derive(Default)]
pub struct PluginsGroup {
    plugins: Vec<Box<dyn EntryPlugin>>,
}

impl PluginsGroup {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn push<P: EntryPlugin + 'static>(&mut self, plugin: P) {
        self.plugins.push(Box::new(plugin));
    }
    pub fn start(&mut self, config: &Config) {
        for plugin in &mut self.plugins {
            plugin.start(&config);
        }
    }
}

impl Iterator for PluginsGroup {
    type Item = ListEntry;
    fn next(&mut self) -> Option<Self::Item> {
        for plugin in &mut self.plugins {
            if let Some(res) = plugin.next() {
                return Some(res);
            }
        }
        None
    }
}
