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
}

pub fn entry_tree(
    base_level: &[ListEntry],
    max_level: usize,
) -> impl Iterator<Item = (usize, &ListEntry)> {
    ListEntryTreeIter::new(base_level, max_level)
}

struct ListEntryTreeIter<'a> {
    queue: Vec<(usize, &'a ListEntry)>,
    max_level: usize,
}

impl<'a> ListEntryTreeIter<'a> {
    pub fn new(base_list: &'a [ListEntry], max_level: usize) -> Self {
        let queue = base_list.iter().rev().map(|ent| (0, ent)).collect();
        Self { queue, max_level }
    }
}

impl<'a> Iterator for ListEntryTreeIter<'a> {
    type Item = (usize, &'a ListEntry);
    fn next(&mut self) -> Option<Self::Item> {
        let (next_level, next_ent) = self.queue.pop()?;
        if next_level < self.max_level {
            for child in next_ent.children.iter().rev() {
                self.queue.push((next_level + 1, child));
            }
        }
        Some((next_level, next_ent))
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
    fn test_tree_iter() {
        let root0: ListEntry = test_ent(
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
        let root1: ListEntry = test_ent(
            "1",
            vec![
                test_ent("10", vec![]),
                test_ent(
                    "11",
                    vec![
                        test_ent("110", vec![]),
                        test_ent("111", vec![]),
                        test_ent("112", vec![]),
                    ],
                ),
                test_ent("12", vec![test_ent("121", vec![test_ent("1211", vec![])])]),
            ],
        );

        let base = [root0, root1];
        let res_level_0 = entry_tree(&base, 0)
            .map(|(lvl, ent)| (lvl, ent.display_name.clone().unwrap()))
            .collect::<Vec<_>>();
        let expected_level_0 = vec![(0, "0".to_owned()), (0, "1".to_owned())];
        assert_eq!(expected_level_0, res_level_0);

        let res_level_1 = entry_tree(&base, 1)
            .map(|(lvl, ent)| (lvl, ent.display_name.clone().unwrap()))
            .collect::<Vec<_>>();
        let expected_level_1 = vec![
            (0, "0".to_owned()),
            (1, "00".to_owned()),
            (1, "01".to_owned()),
            (1, "02".to_owned()),
            (0, "1".to_owned()),
            (1, "10".to_owned()),
            (1, "11".to_owned()),
            (1, "12".to_owned()),
        ];
        assert_eq!(expected_level_1, res_level_1);

        let (max_level, count) = entry_tree(&base, usize::max_value()).fold(
            (0, 0),
            |(cur_max, cur_count), (lvl, _ent)| {
                let next_max = cur_max.max(lvl);
                let next_count = cur_count + 1;
                (next_max, next_count)
            },
        );
        assert_eq!(max_level, 3);
        assert_eq!(count, 18);
    }

    fn test_ent(name: impl AsRef<str>, children: Vec<ListEntry>) -> ListEntry {
        ListEntry {
            display_name: Some(name.as_ref().to_owned()),
            children,
            ..Default::default()
        }
    }
}
