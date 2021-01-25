use crate::config::Config;

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign};
use std::{cmp::Ordering, path::Path};

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

pub fn entry_tree_with_paths(
    base_level: &[ListEntry],
    max_level: usize,
) -> impl Iterator<Item = (EntryPath, &ListEntry)> {
    let mut queue: Vec<_> = base_level
        .iter()
        .enumerate()
        .rev()
        .map(|(idx, ent)| (EntryPath::new().then(idx), ent))
        .collect();
    std::iter::from_fn(move || {
        let (next_path, next_ent) = queue.pop()?;
        if next_path.level().saturating_sub(1) < max_level {
            for (idx, child) in next_ent.children.iter().enumerate().rev() {
                queue.push((next_path.then(idx), child));
            }
        }
        Some((next_path, next_ent))
    })
}

pub fn entry_tree_get(base_level: &[ListEntry], path: EntryPath) -> Option<&ListEntry> {
    let mut cur_level = base_level;
    let mut path_iter = path.iter();
    let mut cur_idx = path_iter.next()?;
    for next_idx in path_iter {
        cur_level = &cur_level.get(cur_idx)?.children;
        cur_idx = next_idx;
    }
    cur_level.get(cur_idx)
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

#[derive(Clone, Copy)]
pub struct EntryPath {
    offsets: [u16; 8],
    level: u8,
}

impl Eq for EntryPath {}
impl PartialEq for EntryPath {
    fn eq(&self, other: &Self) -> bool {
        self.offsets[..self.level()] == other.offsets[..other.level()]
    }
}

impl Hash for EntryPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.offsets[..self.level()]).hash(state)
    }
}
impl From<Vec<usize>> for EntryPath {
    fn from(raw: Vec<usize>) -> Self {
        let mut retvl = Self::new();
        for idx in raw {
            retvl = retvl.then(idx);
        }
        retvl
    }
}

impl fmt::Debug for EntryPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntryPath")
            .field("offsets", &&self.offsets[..self.level()])
            .finish()
    }
}

impl Add for EntryPath {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        let mut nxt = self;
        nxt += rhs;
        nxt
    }
}

impl AddAssign for EntryPath {
    fn add_assign(&mut self, rhs: Self) {
        for other in rhs.iter() {
            self.push(other);
        }
    }
}

impl EntryPath {
    const EMPTY_VALUE: u16 = u16::max_value();

    pub fn new() -> Self {
        Self {
            offsets: [Self::EMPTY_VALUE; 8],
            level: 0,
        }
    }
    fn push(&mut self, next: usize) {
        self.offsets[self.level as usize] = next as u16;
        self.level += 1;
    }
    pub fn then(&self, next: usize) -> Self {
        let mut nxt = *self;
        nxt.push(next);
        nxt
    }
    pub fn level(&self) -> usize {
        self.level as usize
    }

    pub fn parent(&self) -> Self {
        if self.level() == 0 {
            *self
        } else {
            let mut next = *self;
            next.level -= 1;
            next
        }
    }
    pub fn prev_sibling(&self) -> Option<Self> {
        let tail = self
            .level()
            .checked_sub(1)
            .and_then(|n| self.offsets.get(n))
            .map(|n| *n as usize)
            .unwrap_or(0);
        if tail == 0 {
            None
        } else {
            Some(self.parent().then(tail - 1))
        }
    }
    pub fn next_sibling(&self) -> Option<Self> {
        let tail = self
            .level()
            .checked_sub(1)
            .and_then(|n| self.offsets.get(n))
            .map(|n| *n as usize)?;
        Some(self.parent().then(tail + 1))
    }
    pub fn tail_from(&self, level: usize) -> Self {
        let mut retvl = Self::new();
        let offsets_range = level.min(self.level as usize)..(self.level as usize);

        let offsets_slice = &self.offsets[offsets_range];
        (&mut retvl.offsets[..offsets_slice.len()]).copy_from_slice(offsets_slice);
        retvl.level = (self.level as usize).saturating_sub(level) as u8;
        retvl
    }


    pub fn iter<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        self.offsets
            .iter()
            .take(self.level as usize)
            .map(|n| *n as usize)
    }
    pub fn cmp_depth_first(&self, other: &Self) -> Ordering {
        let mut self_iter = self.iter();
        let mut other_iter = other.iter();
        loop {
            let self_next = self_iter.next();
            let other_next = other_iter.next();
            match (self_next, other_next) {
                (Some(a), Some(b)) if a == b => {
                    continue;
                }
                (Some(a), Some(b)) => {
                    return a.cmp(&b);
                }
                (None, Some(_)) => {
                    return Ordering::Less;
                }
                (Some(_), None) => {
                    return Ordering::Greater;
                }
                (None, None) => {
                    return Ordering::Equal;
                }
            }
        }
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



        let with_paths_0 = entry_tree_with_paths(&base, 0)
            .map(|(lvl, ent)| (lvl, ent.display_name.clone().unwrap()))
            .collect::<Vec<_>>();

        assert_eq!(
            vec![
                (vec![0].into(), "0".to_owned()),
                (vec![1].into(), "1".to_owned()),
            ],
            with_paths_0
        );

        let with_paths_1 = entry_tree_with_paths(&base, 1)
            .map(|(lvl, ent)| (lvl, ent.display_name.clone().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(
            vec![
                (vec![0].into(), "0".to_owned()),
                (vec![0, 0].into(), "00".to_owned()),
                (vec![0, 1].into(), "01".to_owned()),
                (vec![0, 2].into(), "02".to_owned()),
                (vec![1].into(), "1".to_owned()),
                (vec![1, 0].into(), "10".to_owned()),
                (vec![1, 1].into(), "11".to_owned()),
                (vec![1, 2].into(), "12".to_owned()),
            ],
            with_paths_1
        );

        let with_paths_full = entry_tree_with_paths(&base, 1024)
            .map(|(lvl, ent)| (lvl, ent.display_name.clone().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(
            vec![
                (vec![0].into(), "0".to_owned()),
                (vec![0, 0].into(), "00".to_owned()),
                (vec![0, 1].into(), "01".to_owned()),
                (vec![0, 1, 0].into(), "010".to_owned()),
                (vec![0, 1, 1].into(), "011".to_owned()),
                (vec![0, 1, 2].into(), "012".to_owned()),
                (vec![0, 2].into(), "02".to_owned()),
                (vec![0, 2, 0].into(), "021".to_owned()),
                (vec![0, 2, 0, 0].into(), "0211".to_owned()),
                (vec![1].into(), "1".to_owned()),
                (vec![1, 0].into(), "10".to_owned()),
                (vec![1, 1].into(), "11".to_owned()),
                (vec![1, 1, 0].into(), "110".to_owned()),
                (vec![1, 1, 1].into(), "111".to_owned()),
                (vec![1, 1, 2].into(), "112".to_owned()),
                (vec![1, 2].into(), "12".to_owned()),
                (vec![1, 2, 0].into(), "121".to_owned()),
                (vec![1, 2, 0, 0].into(), "1211".to_owned()),
            ],
            with_paths_full
        );
    }

    fn test_ent(name: impl AsRef<str>, children: Vec<ListEntry>) -> ListEntry {
        ListEntry {
            display_name: Some(name.as_ref().to_owned()),
            children,
            ..Default::default()
        }
    }

    #[test]
    fn test_pathing() {
        let base = EntryPath::new().then(10).then(21).then(2).then(43);
        assert_eq!(4, base.level());
        assert_eq!(vec![10, 21, 2, 43], base.iter().collect::<Vec<_>>());

        assert_eq!(
            vec![21, 2, 43],
            base.tail_from(1).iter().collect::<Vec<_>>()
        );

        assert_eq!(
            vec![5, 6, 7, 8],
            EntryPath::from(vec![5, 6, 7, 8]).iter().collect::<Vec<_>>()
        );
    }
}
