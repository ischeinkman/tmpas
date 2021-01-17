use crate::{model::ListEntry, AppMessage, UiMessage};
use std::borrow::Cow;
use std::io::{self, Write};

use crossterm::event;
use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, style, ExecutableCommand, QueueableCommand};
use io::Stdout;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

pub struct UiState {
    stdout: Stdout,
    current_results: Vec<ListEntry>,
    screen_offset: usize,
    selection_position: usize,
    search_buffer: SearchBuffer,
}

#[derive(Debug, Default)]
struct SearchBuffer {
    pub buffer: String,
    pub cursor_position: usize,
}

impl SearchBuffer {
    pub fn move_left(&mut self) {
        let cur_str = &self.buffer[..self.cursor_position];
        let last_char = cur_str.chars().last();
        let char_size = last_char.map_or(0, |c| c.len_utf8());
        self.cursor_position = self.cursor_position.saturating_sub(char_size);
    }
    pub fn move_right(&mut self) {
        if self.cursor_position == self.buffer.len() {
            return;
        }
        let cur_str = &self.buffer[self.cursor_position..];
        let next_char = cur_str.chars().next().map_or(0, |c| c.len_utf8());
        self.cursor_position = self
            .cursor_position
            .saturating_add(next_char)
            .min(self.buffer.len());
    }
    pub fn push(&mut self, c: char) {
        self.buffer.insert(self.cursor_position, c);
        self.move_right();
    }
    pub fn backspace(&mut self) {
        if self.cursor_position == 0 {
        } else if self.cursor_position == self.buffer.len() {
            self.buffer.pop();
            self.cursor_position = self.buffer.len();
        } else {
            self.buffer.remove(self.cursor_position);
            self.move_left();
        }
    }
    pub fn delete(&mut self) {
        if self.cursor_position == self.buffer.len() {
            return;
        }
        self.move_right();
        self.backspace();
    }
    pub fn height(&self) -> u16 {
        3
    }
    pub fn display(&mut self, output: &mut impl Write) -> crossterm::Result<()> {
        let (width, _) = terminal::size()?;
        output
            .queue(cursor::MoveTo(0, 0))?
            .queue(style::Print("-".repeat(width.into())))?;
        output
            .queue(cursor::MoveTo(0, 1))?
            .queue(cursor::MoveTo(0, 1))?
            .queue(style::Print("| Search: "))?;
        output.flush()?;
        output.queue(cursor::SavePosition)?;
        output
            .queue(style::Print(&self.buffer))?
            .queue(cursor::MoveTo(width - 2, 1))?
            .queue(style::Print(" |"))?;
        output
            .queue(cursor::MoveTo(0, 2))?
            .queue(style::Print("-".repeat(width.into())))?;
        output.flush()?;
        output.queue(cursor::RestorePosition)?;
        output.queue(cursor::MoveRight(self.cursor_position as u16))?;
        output.queue(cursor::Show)?;
        Ok(())
    }
}

impl UiState {
    pub fn new() -> crossterm::Result<Self> {
        let stdout = io::stdout();
        stdout
            .lock()
            .execute(terminal::EnterAlternateScreen)?
            .execute(terminal::DisableLineWrap)?
            .execute(cursor::Hide)?;
        terminal::enable_raw_mode()?;
        Ok(Self {
            stdout,
            search_buffer: SearchBuffer::default(),
            current_results: Vec::new(),
            screen_offset: 0,
            selection_position: 0,
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
                let idx = self
                    .selection_position
                    .checked_sub(1)
                    .map(|n| n + self.screen_offset);
                let idx = match idx {
                    Some(n) => n,
                    None => {
                        return Ok(None);
                    }
                };
                let result = self.current_results.get(idx).cloned().unwrap();
                Ok(Some(UiMessage::RunEntry(result)))
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
                if self.selection_position > 1 {
                    self.selection_position -= 1;
                } else if self.screen_offset > 0 {
                    self.screen_offset -= 1;
                } else {
                    self.selection_position = 0;
                }
                Ok(None)
            }
            KeyCode::Down => {
                let (_, height) = terminal::size()?;
                if self.selection_position.saturating_add(self.search_buffer.height() as usize + 1) < usize::from(height) {
                    self.selection_position += 1;
                } else if self.screen_offset + 1 < self.current_results.len() {
                    self.screen_offset += 1;
                } else {
                    self.selection_position = 0;
                }
                Ok(None)
            }
            _other => Ok(None),
        }
    }

    pub fn send_message(&mut self, app_msg: AppMessage) {
        match app_msg {
            AppMessage::SearchResults(res) => {
                self.current_results = res;
                self.screen_offset = 0;
                self.selection_position = 0;
            }
        }
    }
    pub fn display(&mut self) -> crossterm::Result<()> {
        let (_width, height) = terminal::size()?;
        let mut stdout = self.stdout.lock();
        stdout
            .queue(cursor::Hide)?
            .queue(terminal::Clear(ClearType::All))?
            .queue(cursor::MoveTo(0, self.search_buffer.height()))?;
        let mut cur_offset = 1u16;
        for ent in self.current_results.iter().skip(self.screen_offset) {
            let selection = self.selection_position.checked_sub(cur_offset.into());
            let next_offset = queue_display_recursive(&mut stdout, ent, 0, selection)?;
            cur_offset += next_offset as u16;
            if cur_offset >= height.saturating_sub(self.search_buffer.height())
                || next_offset == 0
            {
                break;
            }
        }
        self.search_buffer.display(&mut stdout)?;
        stdout.flush()?;
        Ok(())
    }
}

impl Drop for UiState {
    fn drop(&mut self) {
        terminal::disable_raw_mode()
            .and_then(|_| {
                self.stdout
                    .lock()
                    .execute(terminal::EnableLineWrap)?
                    .execute(terminal::LeaveAlternateScreen)?
                    .execute(cursor::Show)?;
                Ok(())
            })
            .unwrap();
    }
}

fn queue_display_recursive(
    output: &mut impl io::Write,
    ent: &ListEntry,
    lvl: usize,
    should_select: Option<usize>,
) -> crossterm::Result<usize> {
    if cursor::position()?.1 >= terminal::size()?.1.saturating_sub(2) {
        return Ok(0);
    }
    let prefix: Cow<'static, str> = match lvl {
        0 => "  ".into(),
        1 => "  |- ".into(),
        other => format!("{}  |- ", "  ".repeat(other)).into(),
    };
    if should_select == Some(0) {
        let content =
            style::style(format!("{}{}", prefix, ent.name())).attribute(style::Attribute::Reverse);
        output.queue(style::PrintStyledContent(content))?;
    } else {
        let content = style::style(format!("{}{}", prefix, ent.name()));
        output.queue(style::PrintStyledContent(content))?;
    }
    output.queue(cursor::MoveToNextLine(1))?;
    let mut offset = 1;
    for child in ent.children.iter() {
        let child_should_select = should_select.and_then(|n| n.checked_sub(offset));
        let child_rows = queue_display_recursive(output, child, lvl + 1, child_should_select)?;
        offset += child_rows;
    }
    Ok(offset)
}
