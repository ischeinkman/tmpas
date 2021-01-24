mod output;
use output::LazyWriter;
mod searchbar;
use resultslist::EntryList;
use searchbar::SearchBuffer;
mod resultslist;

use crate::State;
use crate::{AppMessage, UiMessage};

use std::io::{self, Write};

use crossterm::event;
use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, ExecutableCommand, QueueableCommand};
use io::Stdout;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

pub fn run(state: State) {
    let mut ui = UiState::new().unwrap();
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

pub struct UiState {
    stdout: LazyWriter<Stdout>,
    results_list: EntryList,
    search_buffer: SearchBuffer,
}

impl UiState {
    pub fn new() -> crossterm::Result<Self> {
        let stdout = io::stdout();
        stdout
            .lock()
            .execute(terminal::EnterAlternateScreen)?
            .execute(terminal::DisableLineWrap)?
            .execute(cursor::Hide)?
            .flush()?;
        terminal::enable_raw_mode()?;
        Ok(Self {
            stdout: LazyWriter::new(stdout),
            search_buffer: SearchBuffer::new(),
            results_list: EntryList::new(),
        })
    }

    pub fn step(&mut self) -> crossterm::Result<Option<UiMessage>> {
        let key_event = match event::read()? {
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                return Ok(Some(UiMessage::Quit));
            }
            Event::Key(KeyEvent { code, .. }) => code,
            _other => {
                return Ok(None);
            }
        };
        match key_event {
            KeyCode::Enter => {
                let result = self
                    .results_list
                    .selected()
                    .cloned()
                    .map(UiMessage::RunEntry);
                Ok(result)
            }
            KeyCode::Backspace => {
                self.search_buffer.backspace();
                Ok(Some(UiMessage::DoSearch(self.search_buffer.buffer.clone())))
            }
            KeyCode::Delete => {
                self.search_buffer.delete();
                Ok(Some(UiMessage::DoSearch(self.search_buffer.buffer.clone())))
            }
            KeyCode::Char(c) => {
                self.search_buffer.push(c);
                Ok(Some(UiMessage::DoSearch(self.search_buffer.buffer.clone())))
            }
            KeyCode::Left => {
                self.search_buffer.move_left();
                Ok(None)
            }
            KeyCode::Right => {
                self.search_buffer.move_right();
                Ok(None)
            }
            KeyCode::Up => {
                self.results_list.cursor_up();
                Ok(None)
            }
            KeyCode::Down => {
                self.results_list.cursor_down()?;
                Ok(None)
            }
            _other => Ok(None),
        }
    }

    pub fn send_message(&mut self, app_msg: AppMessage) {
        match app_msg {
            AppMessage::SearchResults(res) => {
                self.results_list.set_results(res);
            }
        }
    }
    pub fn display(&mut self) -> crossterm::Result<()> {
        self.stdout
            .queue(cursor::Hide)?
            .queue(terminal::Clear(ClearType::All))?
            .queue(cursor::MoveTo(0, self.search_buffer.height()))?;
        self.results_list.display(&mut self.stdout)?;
        self.search_buffer.display(&mut self.stdout)?;
        self.stdout.flush()?;
        Ok(())
    }

    fn on_drop(&mut self) -> crossterm::Result<()> {
        terminal::disable_raw_mode()?;
        self.stdout
            .execute(terminal::EnableLineWrap)?
            .execute(terminal::LeaveAlternateScreen)?
            .execute(cursor::Show)?
            .flush()?;
        Ok(())
    }
}

impl Drop for UiState {
    fn drop(&mut self) {
        self.on_drop().unwrap();
    }
}
