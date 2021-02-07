use super::styling::EntryListConfig;
use super::ActionResponse;
use super::KeyAction;
use super::Rect;
use crate::model::{entry_tree_with_paths, ListEntry};

use andrew::shapes::rectangle::Rectangle;
use andrew::text::Text;
use andrew::Canvas;

#[derive(Debug)]
pub struct EntryList {
    config: EntryListConfig,
    current_results: Vec<ListEntry>,
    screen_offset: usize,
    selection_position: usize,
}

impl EntryList {
    pub fn new(config: EntryListConfig) -> Self {
        Self {
            config,
            current_results: Vec::new(),
            screen_offset: 0,
            selection_position: 0,
        }
    }
    pub fn set_results(&mut self, new_results: Vec<ListEntry>) {
        self.current_results = new_results;
        self.screen_offset = 0;
        self.selection_position = self.current_results.len().min(1);
    }
    pub fn max_entries(&self) -> usize {
        let max_height: usize = 1080;
        max_height / self.config.entry_height()
    }
    pub fn cur_results_height(&self) -> usize {
        entry_tree_with_paths(&self.current_results, 1024).count()
    }
    pub fn selected(&self) -> Option<&ListEntry> {
        let idx = self.selection_position.checked_sub(1)?;
        entry_tree_with_paths(&self.current_results, 1024)
            .map(|(_, ent)| ent)
            .nth(idx)
    }

    pub fn buffer_height(&self) -> usize {
        self.cur_results_height().saturating_sub(self.screen_offset)
    }
    pub fn set_buffer(&mut self, expanded_results : Vec<ListEntry>) {
        self.current_results = expanded_results;
    }
    pub fn push_action(&mut self, action: KeyAction) -> ActionResponse {
        match action {
            KeyAction::Up => {
                let nxt = self.selection_position.saturating_sub(1);
                if nxt != self.selection_position {
                    self.selection_position = nxt;
                    eprintln!("Pos: {:?}", self.selection_position);
                    ActionResponse::NeedsRedraw
                } else {
                    ActionResponse::Handled
                }
            }
            KeyAction::Down => {
                if self.selection_position >= self.cur_results_height() {
                    self.selection_position = 0;
                    eprintln!("Pos: {:?}", self.selection_position);
                } else {
                    self.selection_position += 1;
                    eprintln!("Pos: {:?}", self.selection_position);
                }
                ActionResponse::NeedsRedraw
            }
            other => ActionResponse::Continue(other),
        }
    }
    pub fn display(&mut self, borders: Rect, output: &mut Canvas) {
        let max_entries = borders.height / self.config.entry_height();
        self.rectify_offset(max_entries);
        let selection = self.selection_position.checked_sub(1);
        let to_draw = entry_tree_with_paths(&self.current_results, 1024)
            .enumerate()
            .skip(self.screen_offset)
            .take(max_entries);

        let font_data = self.config.font_data.get_font().unwrap();
        for (idx, (path, ent)) in to_draw {
            let display_idx = idx - self.screen_offset;
            let y = display_idx * self.config.entry_height() + borders.y;

            let level = path.level() - 1;
            let prefix_padding = self.config.prefix_size(level);
            let x = borders.x + prefix_padding;

            let w = borders.width - prefix_padding;
            let h = self.config.font_size.ceil() as usize;

            let is_selected = selection == Some(idx);
            let bg = self.config.background_color(path, ent, is_selected);
            let bg_rect = Rectangle::new((x, y), (w, h), None, Some(bg));

            let fg = self.config.text_color(path, ent, is_selected);
            let entry_name = ent.name();
            let mut text = Text::new(
                (x, y),
                fg,
                font_data,
                self.config.font_size,
                1.0,
                entry_name,
            );
            if text.get_width() > w {
                let shrink_factor = (w as f32) / (text.get_width() as f32);
                let old_len = entry_name.chars().count();
                let new_len = ((old_len as f32) * shrink_factor).floor() as usize;
                let new_byte_len = entry_name
                    .char_indices()
                    .map(|(idx, _)| idx)
                    .nth(new_len)
                    .unwrap_or_else(|| entry_name.len());
                let mut entry_name = entry_name[..new_byte_len].to_owned();
                if new_len > 2 {
                    entry_name.pop();
                    entry_name.pop();
                    entry_name.push_str("..");
                }
                text = Text::new(
                    (x, y),
                    fg,
                    font_data,
                    self.config.font_size,
                    1.0,
                    entry_name,
                );
            }
            output.draw(&bg_rect);
            output.draw(&text);
        }
    }

    fn rectify_offset(&mut self, max_entries: usize) {
        let first_invisible = self.screen_offset + max_entries;
        let selected_idx = self.selection_position.saturating_sub(1);
        if selected_idx < self.screen_offset {
            self.screen_offset = selected_idx
        } else if selected_idx >= first_invisible {
            self.screen_offset = selected_idx.saturating_sub(max_entries.saturating_sub(1));
        }
    }
}
