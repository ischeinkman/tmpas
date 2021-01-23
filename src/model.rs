use crate::config::Config;

use std::path::Path;

pub trait EntryPlugin {
    fn name(&self) -> String;
    fn start(&mut self, config: &Config);
    fn next(&mut self) -> Option<ListEntry>;
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Default)]
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
                .sum::<usize>()
                .saturating_add(1),
        }
    }

    pub fn get_leaf(&self, level: usize, idx: usize) -> Option<&ListEntry> {
        if idx == 0 {
            return Some(self);
        }
        else if level == 0 {
            return None;
        }
        let mut child_offset = 1;
        for child in self.children.iter() {
            let child_len = child.expanded_length(level - 1);
            if idx >= child_offset + child_len {
                child_offset += child_len;
                continue;
            }
            else {
                let child_idx = idx - child_offset;
                return child.get_leaf(level - 1, child_idx);
            }
        }
        None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expanded_length() {
        let tree: ListEntry = test_ent(
            "0",
            vec![
                test_ent("00", vec![]),
                test_ent(
                    "01",
                    vec![
                        test_ent("010", vec![]),
                        test_ent("011", vec![]),
                        test_ent("012", vec![]),
                    ],
                ),
                test_ent("02", vec![test_ent("021", vec![test_ent("0211", vec![])])]),
            ],
        );

        assert_eq!(1, tree.expanded_length(0));
        assert_eq!(4, tree.expanded_length(1));
        assert_eq!(8, tree.expanded_length(2));
        assert_eq!(9, tree.expanded_length(3));
        assert_eq!(9, tree.expanded_length(4));
        assert_eq!(9, tree.expanded_length(5));
    }
    #[test]
    fn test_get_leaf() {
        let tree: ListEntry = test_ent(
            "0",
            vec![
                test_ent("00", vec![]),
                test_ent(
                    "01",
                    vec![
                        test_ent("010", vec![]),
                        test_ent("011", vec![]),
                        test_ent("012", vec![]),
                    ],
                ),
                test_ent("02", vec![test_ent("021", vec![test_ent("0211", vec![])])]),
            ],
        );
        for level in 0..6 {
            let res = tree.get_leaf(level, 0).and_then(|n| n.display_name.as_deref());
            assert_eq!(Some("0"), res, "Level : {}", level);
        }

        let res_11 = tree.get_leaf(1, 1).and_then(|n| n.display_name.as_deref());
        assert_eq!(Some("00"), res_11);
        
        let res_12 = tree.get_leaf(1, 2).and_then(|n| n.display_name.as_deref());
        assert_eq!(Some("01"), res_12);

        let res_13 = tree.get_leaf(1, 3).and_then(|n| n.display_name.as_deref());
        assert_eq!(Some("02"), res_13);

    }

    fn test_ent(name: impl AsRef<str>, children: Vec<ListEntry>) -> ListEntry {
        ListEntry {
            display_name: Some(name.as_ref().to_owned()),
            children,
            ..Default::default()
        }
    }
}
