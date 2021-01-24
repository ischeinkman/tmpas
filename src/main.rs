mod utils;

mod model;
use model::ListEntry;

mod state;
use state::State;

mod config;
use config::Config;

mod tui;

#[cfg(feature = "iced-ui")]
mod icedui;

mod plugins;

fn main() {
    let argv = std::env::args_os().collect::<Vec<_>>();
    let config_path = if argv.len() > 1 {
        argv.last().map(std::path::Path::new)
    } else {
        None
    };
    let config: Config = config_path
        .and_then(|pt| std::fs::read_to_string(pt).ok())
        .and_then(|st| toml::de::from_str(&st).ok())
        .unwrap_or_default();
    eprintln!("CONFIG: {:?}", config);
    let mut state = State::new(config);
    state.start();

    if argv.contains(&"--tui".to_owned().into()) || cfg!(not(feature = "iced-ui")) {
        tui::run(state);
    } else {
        #[cfg(feature = "iced-ui")]
        icedui::run(state);
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum UiMessage {
    DoSearch(String),
    RunEntry(ListEntry),
    Quit,
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum AppMessage {
    SearchResults(Vec<ListEntry>),
}
