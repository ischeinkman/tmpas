use crate::model::ListEntry;

pub struct Config {
    pub list_size: usize,
    pub terminal_runner: String,
    pub language: Option<String>,
}

impl Config {
    pub fn make_terminal_command(&self, entry: &ListEntry) -> String {
        let binary = entry.exec_name().unwrap();
        let flags = entry
            .exec_command
            .iter()
            .skip(1)
            .fold(String::new(), |acc, cur| format!("{} {}", acc, cur));
        let command = format!("{} {}", binary, flags);
        let subs = [
            ("$DISPLAY_NAME", entry.name()),
            ("$BINARY", binary),
            ("$FLAGS", &flags),
            ("$COMMAND", &command),
        ];
        let mut raw = self.terminal_runner.clone();
        for (k, v) in &subs {
            raw = raw.replace(k, v);
        }
        raw
    }
}
