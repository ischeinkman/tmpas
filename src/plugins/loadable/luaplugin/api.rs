use mlua::{Lua, Value as LuaValue};

pub const STATE_KEY: &str = "__PLUGIN_STATE__";

pub fn register(env: &Lua) -> mlua::Result<()> {
    env.globals()
        .set("entry", env.create_function(lua_entry_cb)?)?;
    env.globals()
        .set("plugin", env.create_function(lua_plugin_cb)?)?;
    Ok(())
}

fn lua_plugin_cb<'lua>(state: &'lua Lua, args: mlua::Table) -> mlua::Result<()> {
    state.globals().raw_set(STATE_KEY, args)?;
    Ok(())
}

fn lua_entry_cb<'a>(state: &'a Lua, args: LuaValue<'a>) -> mlua::Result<mlua::Table<'a>> {
    let allowed_keys = ["search_terms", "children", "exec_flags", "name", "exec"];

    let retvl = state.create_table()?;
    retvl.raw_set("search_terms", state.create_table()?)?;
    retvl.raw_set("children", state.create_table()?)?;

    let run_flags = state.create_table()?;
    run_flags.raw_set("is_term", false)?;
    run_flags.raw_set("should_fork", false)?;
    retvl.raw_set("exec_flags", run_flags)?;

    match args {
        LuaValue::String(execstr) => {
            retvl.raw_set("exec", execstr)?;
            Ok(retvl)
        }
        LuaValue::Table(tbl) => {
            for pair in tbl.pairs::<mlua::String, LuaValue>() {
                let (key, val) = pair?;
                if !allowed_keys.contains(&key.to_str()?) {
                    let msg = format!("Error: cannot convert args to entry: Expected fields {:?}, but found unknown field {}.", allowed_keys, key.to_str()?);
                    return Err(mlua::Error::RuntimeError(msg));
                }
                retvl.raw_set(key, val)?;
            }
            let is_okay = matches!(retvl.raw_get("exec")?, LuaValue::String(_));
            if !is_okay {
                let msg =
                    "Error: cannot convert args to entry: Required field \"exec\" is missing."
                        .to_owned();
                return Err(mlua::Error::RuntimeError(msg));
            }
            Ok(retvl)
        }
        other => Err(mlua::Error::RuntimeError(format!(
            "Error: cannot convert args to entry: Expected either exec string or fields, got {}.",
            other.type_name()
        ))),
    }
}
