mod freedesktop;
use freedesktop::FreedesktopPlugin;

mod rawpath;
use rawpath::RawPathPlugin;

mod utils;

mod model;
use model::{Config, ListEntry};

mod state;
use state::State;

mod tui;

fn main() {
    let config = Config {
        list_size: 10,
        language: Some("en".to_owned()),
        terminal_runner: "alacritty --title $DISPLAY_NAME --command $COMMAND".to_owned(),
    };

    let xdgp = FreedesktopPlugin::new();
    let ptp = RawPathPlugin::new();
    let mut state = State::new(config);
    state.push_plugin(xdgp);
    state.push_plugin(ptp);
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
                ent.run(&state.config);
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
