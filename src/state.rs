use crate::model::{ EntryPlugin, ListEntry, };
use crate::config::Config;


use std::collections::HashMap;
use std::ffi::CString;
use nix::unistd::{ForkResult, fork, execvp};

pub struct State {
    pub config: Config,
    entries: HashMap<String, ListEntry>,
    plugins: Vec<Box<dyn EntryPlugin>>,
}

fn matches_search(key: &str, ent: &ListEntry) -> bool {
    ent.name().to_lowercase().contains(key)
        || ent
            .search_terms
            .iter()
            .any(|term| term.to_lowercase().contains(key))
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            entries: HashMap::new(),
            plugins: Vec::new(),
        }
    }
    pub fn push_plugin<P: EntryPlugin + 'static>(&mut self, plugin: P) {
        self.plugins.push(Box::new(plugin));
    }
    pub fn start(&mut self) {
        for plugin in &mut self.plugins {
            plugin.start(&self.config);
        }
        while let Some(()) = self.load_next_entry() {}
    }
    pub fn search_loaded(&mut self, key: &str) -> Vec<ListEntry> {
        let mut retvl = Vec::new();
        let mut retlen = 0;
        for ent in self.entries.values() {
            if matches_search(key, ent) {
                retlen += ent.expanded_length(1);
                retvl.push(ent.clone());
            } else {
                for child in ent
                    .children
                    .iter()
                    .filter(|child| matches_search(key, child))
                {
                    retvl.push(child.clone());
                    retlen += 1;
                }
            }
            if retlen >= self.config.list_size {
                break;
            }
        }

        retvl
    }

    pub fn all_entries(&self) -> Vec<ListEntry> {
        self.entries.values().cloned().collect()
    }

    fn load_next_entry(&mut self) -> Option<()> {
        let ent = self.plugins.iter_mut().find_map(|plugin| plugin.next());
        let mut ent = match ent {
            Some(n) => n,
            None => {
                return None;
            }
        };
        if let Some(cur_ent) = self.entries.get_mut(ent.exec_name().unwrap()) {
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
            }
        } else {
            self.entries
                .insert(ent.exec_name().unwrap().to_owned(), ent);
        }
        Some(())
    }
    #[allow(dead_code)]
    pub fn run(&self, ent : &ListEntry) {
        let binary: &str = match ent.exec_name() {
            Some(n) => n,
            None => {
                return;
            }
        };
        let (fname, argv) = if ent.exec_flags.is_term() {
            let raw = self.config.make_terminal_command(&ent);
            let argv: Vec<_> = raw
                .split(' ')
                .map(|part| CString::new(part).unwrap())
                .collect();
            let fname = argv.first().cloned().unwrap();
            (fname, argv)
        } else {
            let binary = CString::new(binary).unwrap();
            let argv: Vec<_> = ent
                .exec_command
                .iter()
                .cloned()
                .map(|part| CString::new(part).unwrap())
                .collect();
            (binary, argv)
        };
        if ent.exec_flags.should_fork() {
            let fork_res = unsafe { fork() };
            match fork_res {
                Ok(ForkResult::Parent { .. }) => {
                    return;
                }
                Ok(ForkResult::Child) => {}
                Err(e) => {
                    panic!("Failed to fork: {:?}", e);
                }
            }
        }
        execvp(&fname, &argv).unwrap();
    }
}