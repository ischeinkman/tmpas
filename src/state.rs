use crate::config::Config;
use crate::model::{EntryPlugin, ListEntry};

use nix::unistd::{execvp, fork, ForkResult};
use std::ffi::CString;

pub struct State {
    pub config: Config,
    entries: Vec<ListEntry>,
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
            entries: Default::default(),
            plugins: Default::default(),
        }
    }
    pub fn start(&mut self) {
        for builtin in &self.config.builtin_plugins {
            self.plugins.push(builtin.load());
        }
        for loaded in &self.config.loaded_plugins {
            self.plugins.push(loaded.load());
        }
        eprintln!(
            "Loaded plugins: {:?}",
            self.plugins.iter().map(|p| p.name()).collect::<Vec<_>>()
        );
        for plugin in &mut self.plugins {
            plugin.start(&self.config);
        }
        while let Some(()) = self.load_next_entry() {}
    }
    pub fn search_loaded(&self, key: &str) -> Vec<ListEntry> {
        let mut retvl = Vec::new();
        let key = key.to_lowercase();
        for ent in self.entries.iter() {
            if matches_search(&key, ent) {
                retvl.push(ent.clone());
            } else {
                for child in ent
                    .children
                    .iter()
                    .filter(|child| matches_search(&key, child))
                {
                    retvl.push(child.clone());
                }
            }
        }

        retvl
    }

    pub fn all_entries(&self) -> Vec<ListEntry> {
        self.entries.clone()
    }

    fn load_next_entry(&mut self) -> Option<()> {
        let ent = self.plugins.iter_mut().find_map(|plugin| plugin.next());
        let ent = match ent {
            Some(n) => n,
            None => {
                return None;
            }
        };
        self.entries.push(ent);
        Some(())
    }
    #[allow(dead_code)]
    pub fn run(&self, ent: &ListEntry) {
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
