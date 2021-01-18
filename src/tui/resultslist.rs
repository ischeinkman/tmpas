use crate::model::ListEntry;

use crossterm::{cursor, style, terminal, QueueableCommand};

use std::borrow::Cow;
use std::io::Write;

#[derive(Default, Debug)]
pub struct EntryList {
    current_results: Vec<ListEntry>,
    screen_offset: usize,
    selection_position: usize,
}

impl EntryList {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_results(&mut self, new_results: Vec<ListEntry>) {
        self.current_results = new_results;
        self.screen_offset = 0;
        self.selection_position = 0;
    }
    pub fn cursor_up(&mut self) {
        if self.selection_position > 1 {
            self.selection_position -= 1;
        } else if self.screen_offset > 0 {
            self.screen_offset -= 1;
        } else {
            self.selection_position = 0;
        }
    }
    pub fn cursor_down(&mut self) -> crossterm::Result<()> {
        let (_, height) = terminal::size()?;
        if self.selection_position.saturating_add(4) < usize::from(height) {
            self.selection_position += 1;
        } else if self.screen_offset + 1 < self.current_results.len() {
            self.screen_offset += 1;
        } else {
            self.selection_position = 0;
        }
        Ok(())
    }
    pub fn selected(&self) -> Option<&ListEntry> {
        let selected_offset = self.selection_position.checked_sub(1)?;
        let idx = self.screen_offset + selected_offset;
        self.current_results.get(idx)
    }
    pub fn display(&mut self, output: &mut impl Write) -> crossterm::Result<()> {
        let (_width, height) = terminal::size()?;
        let mut cur_offset = 1u16;
        for ent in self.current_results.iter().skip(self.screen_offset) {
            let selection = self.selection_position.checked_sub(cur_offset.into());
            let next_offset = queue_display_recursive(output, ent, 0, selection)?;
            cur_offset += next_offset as u16;
            if cur_offset >= height.saturating_sub(3) || next_offset == 0 {
                break;
            }
        }
        Ok(())
    }
}

fn queue_display_recursive(
    output: &mut impl Write,
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
