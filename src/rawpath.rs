use std::borrow::Cow;
use std::env;
use std::fs::read_dir;
use std::io::{self};
use std::iter;
use std::path::{Path, PathBuf};

use nix::unistd::{access, AccessFlags};

use crate::config::Config;
use crate::model::{EntryPlugin, ListEntry, RunFlags};
use crate::utils::{filter_log, EitherOps};

pub struct RawPathPlugin {
    inner: Box<dyn Iterator<Item = ListEntry>>,
}

impl RawPathPlugin {
    pub fn new() -> Self {
        Self {
            inner: Box::new(None.into_iter()),
        }
    }
}

impl EntryPlugin for RawPathPlugin {
    fn start(&mut self, _config: &Config) {
        let iter = binaries()
            .filter_map(filter_log(|e| {
                eprintln!("ERROR From path variable: {:?}", e);
            }))
            .map(make_entry);
        self.inner = Box::new(iter);
    }
    fn name(&self) -> String {
        "Raw $PATH Variable".to_owned()
    }
    fn next(&mut self) -> Option<ListEntry> {
        self.inner.next()
    }
}

fn make_entry(raw_path: impl AsRef<Path>) -> ListEntry {
    let path_str = match raw_path.as_ref().to_string_lossy() {
        Cow::Borrowed(s) => s.to_owned(),
        Cow::Owned(s) => s,
    };
    ListEntry {
        display_name: None,
        exec_command: vec![path_str],
        exec_flags: RunFlags::new(),
        search_terms: Vec::new(),
        children: Vec::new(),
    }
}

fn binaries() -> impl Iterator<Item = io::Result<PathBuf>> {
    root_folders().flat_map(|root| {
        let entiter = match read_dir(&root) {
            Ok(t) => t,
            Err(e) => {
                return iter::once(Err(e)).right();
            }
        };
        entiter
            .map(|ent_res| ent_res.map(|ent| ent.path()))
            .filter_map(|pt_res| match pt_res {
                Ok(pt) if can_execute(&pt) => Some(Ok(pt)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            })
            .left()
    })
}

fn root_folders() -> impl Iterator<Item = PathBuf> {
    let raw_path = match env::var_os("PATH") {
        Some(p) => p,
        None => {
            return None.into_iter().right();
        }
    };
    let split = env::split_paths(&raw_path);
    split.collect::<Vec<_>>().into_iter().left()
}

fn can_execute<P: AsRef<Path>>(pt: P) -> bool {
    access(pt.as_ref(), AccessFlags::X_OK).is_ok()
}
