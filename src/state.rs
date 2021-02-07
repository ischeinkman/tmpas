use crate::model::{EntryPath, EntryPlugin, ListEntry};
use crate::{config::Config, model::entry_tree_with_paths};

use nix::unistd::{execvp, fork, ForkResult};
use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::hash::Hash;

pub struct State {
    pub config: Config,
    entries: Vec<ListEntry>,
    entries_by_cmd: HashMap<Vec<String>, Vec<DedupMetadata>>,
    plugins: Vec<Box<dyn EntryPlugin>>,
    delete_queue: Vec<EntryPath>,
}

fn matches_search(key: &str, ent: &ListEntry) -> bool {
    if key.is_empty() {
        return true;
    }
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
            entries_by_cmd: Default::default(),
            delete_queue: Default::default(),
        }
    }
    pub fn start(&mut self) {
        for builtin in &self.config.builtin_plugins {
            self.plugins.push(builtin.load());
        }
        for loaded in &self.config.loaded_plugins {
            self.plugins.push(loaded.load());
        }
        for plugin in &mut self.plugins {
            plugin.start(&self.config);
        }
        while let Some(()) = self.load_next_entry() {}
        self.delete_queued();
    }
    fn search_loaded(&mut self, key: &str, max_height: usize) -> Vec<ListEntry> {
        let mut retvl = Vec::new();
        let mut height = 0;
        let key = key.to_lowercase();
        for ent in self.entries.iter() {
            if matches_search(&key, ent) {
                retvl.push(ent.clone());
                height += entry_tree_with_paths(std::slice::from_ref(ent), 1024).count();
                if height >= max_height {
                    return retvl;
                }
            } else {
                for child in ent
                    .children
                    .iter()
                    .filter(|child| matches_search(&key, child))
                {
                    retvl.push(child.clone());
                    height += entry_tree_with_paths(std::slice::from_ref(child), 1024).count();
                    if height >= max_height {
                        return retvl;
                    }
                }
            }
        }

        retvl
    }

    fn cur_search_height(&self, key: &str) -> usize {
        let mut retvl = 0;
        let key = key.to_lowercase();
        for ent in self.entries.iter() {
            if matches_search(&key, ent) {
                retvl += entry_tree_with_paths(std::slice::from_ref(ent), 1024).count();
            } else {
                for child in ent
                    .children
                    .iter()
                    .filter(|child| matches_search(&key, child))
                {
                    retvl += entry_tree_with_paths(std::slice::from_ref(child), 1024).count();
                }
            }
        }
        retvl
    }

    pub fn search(&mut self, key: &str, max_height: usize) -> Vec<ListEntry> {
        const BATCH_SIZE: usize = 30;
        let mut finished_loading = false;
        loop {
            for _ in 0..BATCH_SIZE {
                if self.load_next_entry().is_none() {
                    finished_loading = true;
                    break;
                }
            }
            self.delete_queued();
            if finished_loading || self.cur_search_height(key) >= max_height {
                return self.search_loaded(key, max_height);
            }
        }
    }

    fn load_next_entry(&mut self) -> Option<()> {
        let ent = self.plugins.iter_mut().find_map(|plugin| plugin.next());
        let ent = match ent {
            Some(n) => n,
            None => {
                return None;
            }
        };
        let root_path = EntryPath::new().then(self.entries.len());
        let tmp = [ent];
        for (path, child) in entry_tree_with_paths(&tmp, 1024) {
            let path = root_path + path.tail_from(1);
            let cmd = child.exec_command.clone();
            let cur_dups = self.entries_by_cmd.entry(cmd).or_default();
            let meta = DedupMetadata::new(path, child);

            let mut idx = cur_dups.len();
            let mut should_push = true;
            while let Some(cur) = idx.checked_sub(1).and_then(|n| cur_dups.get(n)) {
                match meta.compare(&cur) {
                    SetCmp::Equal | SetCmp::Subset => {
                        self.delete_queue.push(path);
                        should_push = false;
                        break;
                    }
                    SetCmp::Superset => {
                        self.delete_queue.push(cur_dups.remove(idx - 1).path);
                    }
                    SetCmp::Disjoint => {}
                }
                idx = idx.saturating_sub(1);
            }
            if should_push {
                cur_dups.push(meta);
            }
        }
        let [ent] = tmp;
        self.entries.push(ent);
        Some(())
    }
    fn delete_queued(&mut self) {
        self.delete_queue
            .sort_unstable_by(|a, b| a.cmp_depth_first(b));
        while let Some(nxt) = self.delete_queue.pop() {
            match self.delete_path(nxt) {
                Some(_ent) => {}
                None => {
                    panic!("Deleting nonexistant.");
                }
            }
        }
    }
    fn delete_path(&mut self, path: EntryPath) -> Option<ListEntry> {
        let mut cur_level = &mut self.entries;
        let mut path_iter = path.iter();
        let mut cur_idx = path_iter.next()?;
        for next_idx in path_iter {
            cur_level = &mut cur_level.get_mut(cur_idx)?.children;
            cur_idx = next_idx;
        }
        if cur_level.len() > cur_idx {
            Some(cur_level.remove(cur_idx))
        } else {
            None
        }
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

#[derive(Debug, PartialEq, Eq)]
struct DedupMetadata {
    path: EntryPath,
    display_name: Option<String>,
    children: usize,
    search_terms: HashSet<String>,
}

impl DedupMetadata {
    pub fn new(path: EntryPath, entry: &ListEntry) -> Self {
        Self {
            path,
            display_name: entry.display_name.as_ref().cloned(),
            children: entry.children.len(),
            search_terms: entry.search_terms.iter().cloned().collect(),
        }
    }
    pub fn has_children(&self) -> bool {
        self.children != 0
    }
    pub fn level(&self) -> usize {
        self.path.level() - 1
    }
    pub fn is_child(&self) -> bool {
        self.level() != 0
    }

    pub fn compare(&self, other: &Self) -> SetCmp {
        let cmp = match (self.display_name.as_deref(), other.display_name.as_deref()) {
            (Some(a), Some(b)) if a != b => SetCmp::Disjoint,
            (Some(_), None) => SetCmp::Superset,
            (None, Some(_)) => SetCmp::Subset,
            (Some(_), Some(_)) | (None, None) => SetCmp::Equal,
        };
        let cmp = cmp.then_with(|| match (self.is_child(), other.is_child()) {
            (true, false) => SetCmp::Superset,
            (false, true) => SetCmp::Superset,
            (true, true) => SetCmp::Disjoint,
            (false, false) => SetCmp::Equal,
        });

        let cmp = cmp.then_with(|| match (self.has_children(), other.has_children()) {
            (true, true) => SetCmp::Disjoint,
            (true, false) => SetCmp::Superset,
            (false, true) => SetCmp::Subset,
            (false, false) => SetCmp::Equal,
        });
        cmp.then_with(|| SetCmp::set_relationship(&self.search_terms, &other.search_terms))
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum SetCmp {
    Superset,
    Subset,
    Equal,
    Disjoint,
}

impl SetCmp {
    pub fn set_relationship<T: Hash + Eq>(lhs: &HashSet<T>, rhs: &HashSet<T>) -> SetCmp {
        if lhs == rhs {
            SetCmp::Equal
        } else if lhs.is_subset(&rhs) {
            SetCmp::Subset
        } else if lhs.is_superset(&rhs) {
            SetCmp::Superset
        } else {
            SetCmp::Disjoint
        }
    }
    pub const fn then(&self, next: SetCmp) -> SetCmp {
        match (self, next) {
            // If we are already disjoint, maintain that disjoint-ness
            (SetCmp::Disjoint, _) | (_, SetCmp::Disjoint) => SetCmp::Disjoint,
            // If one field has a > relation and the other has a <, we lose comparibility
            // and the relation becomes disjoint
            (SetCmp::Superset, SetCmp::Subset) | (SetCmp::Subset, SetCmp::Superset) => {
                SetCmp::Disjoint
            }
            (SetCmp::Subset, _) | (SetCmp::Equal, SetCmp::Subset) => SetCmp::Subset,
            (SetCmp::Superset, _) | (SetCmp::Equal, SetCmp::Superset) => SetCmp::Superset,
            (SetCmp::Equal, SetCmp::Equal) => SetCmp::Equal,
        }
    }

    pub fn then_with<F: FnOnce() -> SetCmp>(&self, next: F) -> SetCmp {
        if *self == SetCmp::Disjoint {
            SetCmp::Disjoint
        } else {
            self.then(next())
        }
    }
}
