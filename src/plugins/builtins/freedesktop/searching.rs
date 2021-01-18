use crate::utils::EitherOps;

use std::fs;
use std::io;
use std::iter;
use std::path::{Path, PathBuf};
use std::{env, ffi::OsStr};

#[allow(dead_code)]
pub fn xdg_desktop_files() -> impl Iterator<Item = Result<PathBuf, io::Error>> {
    let base_data_dirs = xdg_data_homes().chain(xdg_data_dirs());
    let application_dirs = base_data_dirs
        .map(|mut data_dir| {
            data_dir.push("applications/");
            data_dir
        })
        .filter(|dir| dir.is_dir());
    application_dirs.flat_map(desktop_files_in_dir)
}

fn xdg_data_homes() -> impl Iterator<Item = PathBuf> {
    let raw_env_val = env::var_os("XDG_DATA_HOME");

    match raw_env_val {
        Some(val) => {
            let split = env::split_paths(&val).collect::<Vec<_>>().into_iter();
            let unified = iter::once(PathBuf::from(val));
            unified.chain(split).left()
        }
        None => {
            let home_dir = env::var_os("HOME").map(PathBuf::from);
            let data_home = home_dir.map(|mut cur| {
                cur.push(".local/share/");
                cur
            });
            data_home.into_iter().right()
        }
    }
}

fn xdg_data_dirs() -> impl Iterator<Item = PathBuf> {
    let raw_env_val = env::var_os("XDG_DATA_DIRS");
    match raw_env_val {
        Some(val) => env::split_paths(&val)
            .collect::<Vec<_>>()
            .into_iter()
            .left(),
        None => {
            let default = iter::once(Path::new("/usr/local/share"))
                .chain(iter::once(Path::new("/usr/share")));
            default.map(|p| p.to_owned()).right()
        }
    }
}

fn desktop_files_in_dir<D: AsRef<Path>>(
    dir: D,
) -> impl Iterator<Item = Result<PathBuf, io::Error>> {
    let ent_iter = match fs::read_dir(&dir) {
        Ok(it) => it,
        Err(e) => {
            return iter::once(Err(e)).right();
        }
    };
    ent_iter
        .map(|ent_res| Ok(ent_res?.path()))
        .filter(|ent_res| match ent_res {
            Ok(p) => p.is_file() && p.extension() == Some(OsStr::new("desktop")),
            Err(_) => true,
        })
        .left()
}
