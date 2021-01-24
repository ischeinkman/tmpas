mod utils;

mod model;
use model::ListEntry;

mod state;
use state::State;

mod config;
use config::{Config, UiTag};

mod tui;

#[cfg(feature = "iced-ui")]
mod icedui;

mod plugins;

use structopt::StructOpt;
use utils::ok_or_log;

use std::path::PathBuf;

fn main() {
    let args = CmdArgs::from_args();
    if args.verify {
        let path = args
            .config
            .expect("Was not given path to config file to verify.");
        let raw = std::fs::read_to_string(path).unwrap();
        let parsed: Config = toml::de::from_str(&raw).unwrap();
        println!("{:?}", parsed);
        return;
    }
    let config: Config = args
        .config
        .as_ref()
        .and_then(|pt| {
            ok_or_log(std::fs::read_to_string(pt), |e| {
                eprintln!("Error reading config: {}", e)
            })
        })
        .and_then(|st| {
            ok_or_log(toml::de::from_str(&st), |e| {
                eprintln!("Error parsing config: {}", e)
            })
        })
        .unwrap_or_default();
    eprintln!("CONFIG: {:?}", config);
    let mut state = State::new(config);
    state.start();

    if args.tui && args.gui {
        panic!("Can't run both the tui and gui at the same time.");
    }
    let ui_to_run = if args.tui {
        UiTag::Crossterm
    } else if args.gui {
        UiTag::Iced
    } else {
        state.config.default_interface()
    };

    if !state.config.is_interface_enabled(ui_to_run) {
        panic!(
            "Cannot run tmpas using interface {:?}: interface is disabled.",
            ui_to_run
        );
    }
    match ui_to_run {
        UiTag::Crossterm => {
            tui::run(state);
        }
        UiTag::Iced => {
            #[cfg(feature = "iced-ui")]
            icedui::run(state);
        }
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

#[derive(Debug, StructOpt)]
#[structopt(name = "tmpas")]
struct CmdArgs {
    /// Path to the config file
    #[structopt(long, parse(from_os_str), required_if("verify", "true"))]
    config: Option<PathBuf>,
    #[structopt(long)]
    tui: bool,
    #[structopt(long)]
    gui: bool,
    #[structopt(long)]
    verify: bool,
}
