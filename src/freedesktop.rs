use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::iter;
use std::mem;
use std::path::{Path};
use std::borrow::Cow;

use crate::utils::EitherOps;

mod parsing;
mod searching;

use parsing::SectionReader;

pub fn get_sections() -> impl Iterator<Item = io::Result<Section>> {
    searching::xdg_desktop_files().flat_map(|path_res| {
        let file_res = path_res.and_then(File::open).map(BufReader::new);
        let file = match file_res {
            Ok(file) => file,
            Err(e) => {
                return iter::once(Err(e)).right();
            }
        };
        let mut reader = SectionReader::new();
        let mut lines = file.lines().peekable();

        iter::from_fn(move || loop {
            let raw_line = lines.next()?;
            let raw_line = match raw_line {
                Ok(l) => l,
                Err(e) => {
                    return Some(Err(e));
                }
            };
            if let Some(next) = reader.push(raw_line.as_ref()) {
                return Some(Ok(next));
            }
            if lines.peek().is_none() {
                return mem::take(&mut reader).finish().map(Ok);
            }
        })
        .left()
    })
}

#[derive(Default, Debug)]
pub struct Section {
    pub header: String,
    pub fields: HashMap<String, FieldValue>,
}

impl Section {
    pub fn new(header: String) -> Self {
        Self {
            header,
            fields: HashMap::new(),
        }
    }

    fn is_blank(&self) -> bool {
        self.header.is_empty() && self.fields.is_empty()
    }

    pub fn is_term(&self) -> bool {
        self.get_field("Terminal")
            .map_or(false, |s| s.starts_with(|c| c == 't' || c == 'T'))
    }

    pub fn get_cmd(&self) -> Option<String> {
        let tryexec = self.get_field("TryExec");
        if let Some(ret) = tryexec {
            return Some(ret.to_owned());
        }
        let mut exec = self.get_field("Exec")?;
        if let Some((idx, '%')) = exec.char_indices().nth_back(1) {
            exec = &exec[..idx -1];
        }
        Some(exec.to_owned())
    }

    pub fn exec_name(&self) -> Option<String> {
        let mut cmd_path = self.get_cmd()?;
        if let Some(first_space) = cmd_path.find(' ') {
            cmd_path.truncate(first_space);
        }
        let pt = Path::new(&cmd_path);
        let res = pt.file_name()?.to_string_lossy();
        let res = match res {
            Cow::Borrowed(s) => s.to_owned(), 
            Cow::Owned(s) => s
        };
        Some(res)
    }

    pub fn name<'a>(&self, lang: Option<&'a str>) -> Option<&str> {
        let ent = self.fields.get("Name")?;
        lang.and_then(|lang| ent.attributes.get(lang))
            .or_else(|| ent.default.as_ref())
            .map(|s| s.as_ref())
    }

    fn get_field<'a>(&self, name: &'a str) -> Option<&str> {
        let ent = self.fields.get(name)?;
        let default = ent.default.as_ref()?;
        Some(default.as_ref())
    }
}

#[derive(Default, Debug)]
pub struct FieldValue {
    pub default: Option<String>,
    pub attributes: HashMap<String, String>,
}