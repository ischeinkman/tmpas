use super::LuaConfig;
use crate::config::Config;
use crate::model::{EntryPlugin, ListEntry, RunFlags};

use anyhow::{Context, Error};
use mlua::{self, FromLua, Lua, Value as LuaValue};

use std::cmp::{Eq, PartialEq};
use std::fs;

mod api;
use api::STATE_KEY;

pub struct LuaPlugin {
    conf: LuaConfig,
    env: Lua,
}

impl LuaPlugin {
    pub fn new(conf: LuaConfig) -> mlua::Result<Self> {
        let env = Lua::new();
        api::register(&env)?;
        Ok(Self { conf, env })
    }
    fn plugin_state(&self) -> mlua::Result<LuaPluginState> {
        let res = self.env.globals().raw_get::<_, LuaPluginState>(STATE_KEY);
        res
    }
    fn start_inner(&mut self, _config: &Config) -> Result<(), Error> {
        let name = self.conf.name.as_deref().unwrap_or("");
        let file = fs::read(&self.conf.file).with_context(|| {
            format!(
                "Error reading lua file plugin {} at {}",
                name,
                self.conf.file.display()
            )
        })?;
        let code = self.env.load(&file);
        code.exec().with_context(|| {
            format!(
                "Error running lua file plugin {} at {}",
                name,
                self.conf.file.display()
            )
        })?;
        self.plugin_state().with_context(|| {
            format!(
                "Error in lua file plugin {} at {}: File is not a TMPAS plugin.",
                name,
                self.conf.file.display()
            )
        })?;
        Ok(())
    }
}

impl EntryPlugin for LuaPlugin {
    fn start(&mut self, config: &Config) {
        if let Err(e) = self.start_inner(config) {
            eprintln!("Could not start plugin {:?}: {:?}", self.name(), e);
        }
    }
    fn name(&self) -> String {
        if let Some(nm) = self.conf.name.as_ref() {
            return nm.clone();
        }
        let plugin_res = self.plugin_state().ok().and_then(|s| s.name());
        if let Some(nm) = plugin_res {
            return nm;
        }
        format!("{}", self.conf.file.display())
    }
    fn next(&mut self) -> Option<ListEntry> {
        let raw = self.plugin_state().and_then(|mut st| st.next());
        match raw {
            Ok(ret) => ret,
            Err(e) => {
                eprintln!("Error from lua plugin {:?} : {:?}", self.name(), e);
                None
            }
        }
    }
}

fn parse_lua_entry(args: LuaValue) -> mlua::Result<ListEntry> {
    let args = match args {
        LuaValue::Table(tbl) => tbl,
        other => {
            return Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "tmpas::ListEntry",
                message: Some("Entry argument must be a table!".into()),
            });
        }
    };
    let display_name: Option<String> = args.get("name")?;
    let search_terms: Vec<String> = args.get("search_terms")?;
    let exec_flags = args.get("exec_flags").and_then(parse_lua_exec_flags)?;
    let raw_children: Option<Vec<LuaValue>> = args.get("children")?;
    let children = raw_children
        .into_iter()
        .flat_map(|c| c.into_iter())
        .map(parse_lua_entry)
        .collect::<Result<Vec<_>, _>>()?;
    let exec_command = parse_command_string(&(args.get::<_, String>("exec")?));
    Ok(ListEntry {
        display_name,
        exec_command,
        exec_flags,
        children,
        search_terms,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandStringState {
    None,
    Backtick,
    SingleQuote,
    DoubleQuote,
}
fn parse_command_string(s: &str) -> Vec<String> {
    let mut cur_state = CommandStringState::None;
    let mut prev_escape = false;
    let mut retvl = Vec::new();
    let mut buffer = String::new();
    for c in s.chars() {
        if prev_escape {
            buffer.push(c);
            prev_escape = false;
            continue;
        }
        if c == '\\' {
            prev_escape = true;
            continue;
        }

        match (c, cur_state) {
            (' ', CommandStringState::None) => {
                if !buffer.is_empty() {
                    retvl.push(buffer);
                }
                buffer = String::new();
            }
            ('"', CommandStringState::DoubleQuote)
            | ('\'', CommandStringState::SingleQuote)
            | ('`', CommandStringState::Backtick) => {
                cur_state = CommandStringState::None;
                if !buffer.is_empty() {
                    retvl.push(buffer);
                }
                buffer = String::new();
            }
            ('"', CommandStringState::None) => {
                cur_state = CommandStringState::DoubleQuote;
            }
            ('\'', CommandStringState::None) => {
                cur_state = CommandStringState::SingleQuote;
            }
            ('`', CommandStringState::None) => {
                cur_state = CommandStringState::Backtick;
            }
            (other, _) => {
                buffer.push(other);
            }
        }
    }
    if !buffer.is_empty() || retvl.is_empty() {
        retvl.push(buffer);
    }
    retvl
}

fn parse_lua_exec_flags(args: LuaValue) -> mlua::Result<RunFlags> {
    let args = match args {
        LuaValue::Table(tbl) => tbl,
        LuaValue::Nil => {
            return Ok(RunFlags::new());
        }
        other => {
            return Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "tmpas::RunFlags",
                message: Some("Runflags must be a table!".into()),
            });
        }
    };
    let mut retvl = RunFlags::new();
    for entry in args.pairs::<String, bool>() {
        let (key, val) = entry?;
        match key.as_str() {
            "is_term" => {
                retvl.set_term(val);
            }
            "should_fork" => {
                retvl.set_should_fork(val);
            }
            other => {
                let msg = format!(
                    "Field {} is not in the allowed field list: [\"is_term\", \"should_fork\"]",
                    other
                );
                return Err(mlua::Error::FromLuaConversionError {
                    from: "table",
                    to: "tmpas::RunFlags",
                    message: Some(msg),
                });
            }
        }
    }
    Ok(retvl)
}

#[derive(Debug)]
struct LuaPluginState<'a> {
    inner: Option<mlua::Table<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PluginStateNext {
    Function,
    Table(u16),
    End,
}

impl<'a> LuaPluginState<'a> {
    pub fn name(&self) -> Option<String> {
        self.inner
            .as_ref()
            .and_then(|tbl| tbl.raw_get::<_, Option<String>>("name").ok())
            .flatten()
    }
    pub fn next(&mut self) -> mlua::Result<Option<ListEntry>> {
        if let Some(nextfn) = self.nextfn() {
            let output = nextfn
                .call::<_, Option<LuaValue>>(())?
                .map(parse_lua_entry)
                .transpose()?;
            if let Some(res) = output {
                return Ok(Some(res));
            } else {
                self.set_flag(PluginStateNext::Table(0))?;
            }
        } else if matches!(self.next_flag(), PluginStateNext::Function) {
            self.set_flag(PluginStateNext::Table(0))?;
        }
        if let PluginStateNext::Table(next_idx) = self.next_flag() {
            let inner = match self.inner.as_ref() {
                Some(tbl) => tbl,
                None => {
                    self.set_flag(PluginStateNext::End)?;
                    return Ok(None);
                }
            };
            let res = inner
                .raw_get::<_, Option<mlua::Table>>("entries")?
                .and_then(|tbl| tbl.get::<_, Option<mlua::Value>>(next_idx + 1).transpose())
                .map(|res| res.and_then(parse_lua_entry))
                .transpose()?;
            match res {
                Some(out) => {
                    self.set_flag(PluginStateNext::Table(next_idx + 1))?;
                    return Ok(Some(out));
                }
                None => {
                    self.set_flag(PluginStateNext::End)?;
                    return Ok(None);
                }
            }
        }
        Ok(None)
    }

    fn next_flag(&self) -> PluginStateNext {
        let inner = match self.inner.as_ref() {
            Some(tbl) => tbl,
            None => {
                return PluginStateNext::End;
            }
        };
        let raw = inner
            .raw_get::<_, LuaValue>("__NEXT_FLAG")
            .unwrap_or(LuaValue::Nil);
        match raw {
            LuaValue::Nil => PluginStateNext::Function,
            LuaValue::Integer(n) => {
                let n = (n % i64::from(u16::max_value())) as u16;
                PluginStateNext::Table(n)
            }
            _ => PluginStateNext::End,
        }
    }
    fn set_flag(&mut self, flag: PluginStateNext) -> mlua::Result<()> {
        if let Some(inner) = self.inner.as_ref() {
            match flag {
                PluginStateNext::End => inner.raw_set("__NEXT_FLAG", false),
                PluginStateNext::Function => inner.raw_remove("__NEXT_FLAG"),
                PluginStateNext::Table(n) => inner.raw_set("__NEXT_FLAG", n),
            }
        } else {
            Ok(())
        }
    }
    fn nextfn(&self) -> Option<mlua::Function<'a>> {
        if !matches!(self.next_flag(), PluginStateNext::Function) {
            return None;
        }
        self.inner.as_ref()?.get("nextfn").ok().flatten()
    }
    fn verify(&self) -> mlua::Result<()> {
        let inner = match self.inner.as_ref() {
            Some(v) => v,
            None => {
                return Ok(());
            }
        };
        inner.get::<_, Option<String>>("name")?;
        let raw_entries: Option<Vec<LuaValue>> = inner.get("entries")?;
        raw_entries
            .into_iter()
            .flat_map(|v| v.into_iter())
            .map(parse_lua_entry)
            .find(|res| res.is_err())
            .transpose()?;
        inner.get::<_, Option<mlua::Function>>("next")?;
        Ok(())
    }
}

impl<'lua> FromLua<'lua> for LuaPluginState<'lua> {
    fn from_lua(lua_value: LuaValue<'lua>, _: &'lua Lua) -> mlua::Result<Self> {
        let inner = match lua_value {
            LuaValue::Table(tbl) => Some(tbl),
            LuaValue::Nil => None,
            other => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: other.type_name(),
                    to: "tmpas::LuaPluginState",
                    message: Some("Error: can only build a plugin from a lua table!".into()),
                });
            }
        };
        let retvl = Self { inner };
        retvl.verify()?;
        Ok(retvl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_cmd() {
        let simple = "/usr/bin/cat mout.txt";
        assert_eq!(
            parse_command_string(simple),
            vec!["/usr/bin/cat".to_owned(), "mout.txt".to_owned()]
        );

        let complex = r#" echo "Hello World!" "My name is 'ilan'" "What is `\\\"yours\"`"? "#;
        assert_eq!(
            parse_command_string(complex),
            vec![
                "echo".to_owned(),
                "Hello World!".to_owned(),
                "My name is 'ilan'".to_owned(),
                "What is `\\\"yours\"`".to_owned(),
                "?".to_owned()
            ]
        );
    }
}
