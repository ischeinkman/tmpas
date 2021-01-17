
mod utils;
mod freedesktop;


fn main() {
    let mut sections = freedesktop::get_sections().collect::<Vec<_>>();
    sections.sort_unstable_by_key(|ent| ent.as_ref().ok().and_then(|ent| ent.exec_name()));
    for section in sections {
        println!("==========");
        let section = match section {
            Ok(s) => s, 
            Err(e) => {
                println!("ERROR: {:?}", e);
                println!("==========");
                continue;}
        };
        println!("Section: {}", section.header);
        println!("Name: {:?}", section.name(None).unwrap());
        println!("Command: {:?}", section.exec_name().unwrap());
        println!("Full Exec: {:?}", section.get_cmd().unwrap());
        println!("Terminal?: {:?}", section.is_term());
        println!("==========");
    }
}
