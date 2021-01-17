use std::ffi::OsStr;

mod freedesktop;
mod rawpath;
mod utils;

fn main() {
    let mut sections = freedesktop::get_sections().collect::<Vec<_>>();
    sections.sort_unstable_by_key(|ent| ent.as_ref().ok().and_then(|ent| ent.exec_name()));
    for section in sections.iter() {
        println!("==========");
        let section = match section {
            Ok(s) => s,
            Err(e) => {
                println!("ERROR: {:?}", e);
                println!("==========");
                continue;
            }
        };
        println!("Section: {}", section.header);
        println!("Name: {:?}", section.name(None).unwrap());
        println!("Command: {:?}", section.exec_name().unwrap());
        println!("Full Exec: {:?}", section.get_cmd().unwrap());
        println!("Terminal?: {:?}", section.is_term());
        println!("==========");
    }
    let mut all_binaries = rawpath::binaries().collect::<Vec<_>>();
    all_binaries.sort_unstable_by_key(|ent| {
        ent.as_ref()
            .ok()
            .and_then(|ent| ent.file_name().map(|n| n.to_owned()))
    });
    for binres in all_binaries {
        let bin = match binres {
            Ok(b) => b,
            Err(e) => {
                println!("ERROR FROM BIN: {:?}", e);
                continue;
            }
        };
        let fname = bin.file_name();
        let exists = sections.iter().any(|sect| {
            let sect = match sect {
                Ok(s) => s,
                Err(_) => {
                    return false;
                }
            };
            let nm = sect.exec_name();
            nm.as_ref().map(|n| OsStr::new(n)) == fname
        });
        if exists {
            continue;
        }
        println!("BIN: {:?}", bin);
    }
}
