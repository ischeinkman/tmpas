use crate::utils::EitherOps;
use nix::unistd::{access, AccessFlags};
use std::env;
use std::fs::read_dir;
use std::io::{self};
use std::iter;
use std::path::{Path, PathBuf};

pub fn binaries() -> impl Iterator<Item = io::Result<PathBuf>> {
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
