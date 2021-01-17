use crate::config::Config;

use std::path::Path;

pub trait EntryPlugin {
    fn name(&self) -> String;
    fn start(&mut self, config: &Config);
    fn next(&mut self) -> Option<ListEntry>;
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ListEntry {
    pub display_name: Option<String>,
    pub search_terms: Vec<String>,
    pub exec_command: Vec<String>,
    pub exec_flags: RunFlags,
    pub children: Vec<ListEntry>,
}

impl ListEntry {
    pub fn name(&self) -> &str {
        self.display_name
            .as_deref()
            .or_else(|| self.exec_name())
            .unwrap_or_default()
    }
    pub fn exec_name(&self) -> Option<&str> {
        let raw = self.exec_command.first()?;
        let as_path = Path::new(raw);
        let stripped = as_path.file_name().and_then(|s| s.to_str());
        Some(stripped.unwrap_or(raw))
    }
    pub fn expanded_length(&self, level: usize) -> usize {
        match level {
            0 => 1,
            level => self
                .children
                .iter()
                .map(|child| child.expanded_length(level - 1))
                .sum(),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct RunFlags(u16);

impl RunFlags {
    const IS_TERM: RunFlags = RunFlags(0x1);
    const SHOULD_FORK: RunFlags = RunFlags(0x2);

    pub fn new() -> Self {
        Self(0)
    }

    pub fn is_term(self) -> bool {
        self.0 & Self::IS_TERM.0 != 0
    }

    pub fn set_term(&mut self, value: bool) {
        if value {
            self.0 |= Self::IS_TERM.0;
        } else {
            self.0 &= !Self::IS_TERM.0;
        }
    }

    pub fn with_term(mut self, value: bool) -> Self {
        self.set_term(value);
        self
    }

    pub fn should_fork(&self) -> bool {
        self.0 & Self::SHOULD_FORK.0 != 0
    }

    #[allow(dead_code)]
    pub fn set_should_fork(&mut self, value: bool) {
        if value {
            self.0 |= Self::SHOULD_FORK.0;
        } else {
            self.0 &= !Self::SHOULD_FORK.0;
        }
    }

    #[allow(dead_code)]
    pub fn with_should_fork(mut self, value: bool) -> Self {
        self.set_should_fork(value);
        self
    }
}
