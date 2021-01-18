mod freedesktop;

mod rawpath;

mod utils;

mod model;
use model::ListEntry;

mod state;
use state::State;

mod config;
use config::Config;

mod tui;

mod dummy;

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

    let mut ui = tui::UiState::new().unwrap();
    ui.send_message(AppMessage::SearchResults(state.all_entries()));
    loop {
        let step_res = ui.display().and_then(|_| ui.step());
        match step_res {
            Ok(Some(UiMessage::DoSearch(key))) => {
                let res = state.search_loaded(&key);
                ui.send_message(AppMessage::SearchResults(res));
            }
            Ok(Some(UiMessage::RunEntry(ent))) => {
                drop(ui);
                state.run(&ent);
                return;
            }
            Ok(Some(UiMessage::Quit)) => {
                return;
            }
            Ok(None) => {}
            Err(e) => {
                panic!("Got error: {:?}", e);
            }
        }
    }
}

#[non_exhaustive]
pub enum UiMessage {
    DoSearch(String),
    RunEntry(ListEntry),
    Quit,
}

#[non_exhaustive]
pub enum AppMessage {
    SearchResults(Vec<ListEntry>),
}
