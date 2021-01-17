use crossterm::{cursor, style, terminal, QueueableCommand};
use std::io::Write;

#[derive(Debug, Default)]
pub struct SearchBuffer {
    pub buffer: String,
    pub cursor_position: usize,
}

impl SearchBuffer {
    pub fn new() -> Self {
        Self::default()
    }
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
            .queue(style::Print("o"))?
            .queue(style::Print("-".repeat(width.saturating_sub(2).into())))?
            .queue(style::Print("o"))?;
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
            .queue(style::Print("o"))?
            .queue(style::Print("-".repeat(width.saturating_sub(2).into())))?
            .queue(style::Print("o"))?;
        output.flush()?;
        output.queue(cursor::RestorePosition)?;
        output.queue(cursor::MoveRight(self.cursor_position as u16))?;
        output.queue(cursor::Show)?;
        Ok(())
    }
}
