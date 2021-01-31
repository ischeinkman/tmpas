use super::ActionResponse;
use super::KeyAction;
use super::Rect;
use crate::model::{entry_tree_with_paths, EntryPath, ListEntry};
use std::{panic::catch_unwind, path::PathBuf};

use andrew::shapes::rectangle::Rectangle;
use andrew::text::fontconfig::FontConfig;
use andrew::text::{load_font_file, Text};
use andrew::Canvas;

use anyhow::{anyhow, Context, Error};

#[derive(Debug)]
pub struct EntryListConfig {
    pub font_size: f32,
    pub font_data: Vec<u8>,
    pub entry_spacing: usize,
}

impl EntryListConfig {
    pub fn new() -> Result<Self, Error> {
        let font_config =
            FontConfig::new().map_err(|_| anyhow!("Could not construct FontConfig."))?;
        let all_fonts = font_config
            .get_fonts()
            .with_context(|| "Error getting font list from config.")?;
        let valid_fonts = all_fonts
            .into_iter()
            .filter_map(|path| {
                let fname = path.file_name()?.to_string_lossy().to_lowercase();
                Some((path, fname))
            })
            .filter(|(_, fname)| {
                !fname.contains("bold") && !fname.contains("italic") && !fname.contains("oblique")
            })
            .filter(|(_, fname)| fname.contains("sans"));
        let default_font_path: PathBuf = valid_fonts
            .map(|(path, _)| path)
            .next()
            .ok_or_else(|| anyhow!("Could not find a default font."))?;
        let font_data = catch_unwind(|| load_font_file(&default_font_path))
            .map_err(|e| {
                e.downcast::<String>()
                    .map(Error::msg)
                    .or_else(|e| e.downcast::<&str>().map(Error::msg))
                    .unwrap_or_else(|_| Error::msg("Unknown panic occurred."))
            })
            .with_context(|| {
                format!(
                    "Error reading font path {} as font data.",
                    default_font_path.display()
                )
            })?;
        let res = Self {
            font_size: 32.0,
            font_data,
            entry_spacing: 8,
        };
        Ok(res)
    }
    pub fn text_color(&self, _path: EntryPath, entry: &ListEntry, selected: bool) -> [u8; 4] {
        let is_term = entry.exec_flags.is_term();
        if selected {
            self.background_color(_path, entry, !selected)
        } else if is_term {
            [255, 45, 128, 64]
        } else {
            [255, 0, 0, 0]
        }
    }
    pub fn background_color(&self, _path: EntryPath, entry: &ListEntry, selected: bool) -> [u8; 4] {
        let is_term = entry.exec_flags.is_term();
        if !selected {
            [192, 255, 255, 255]
        } else if is_term {
            [255, 64, 192, 80]
        } else {
            [255, 140, 140, 150]
        }
    }
    pub fn entry_height(&self) -> usize {
        self.font_size.ceil() as usize + self.entry_spacing
    }
    pub fn prefix_size(&self, level: usize) -> usize {
        level * (self.font_size as usize)
    }
}

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
        self.selection_position = 0;
    }
    pub fn selected(&self) -> Option<&ListEntry> {
        let idx = self.selection_position.checked_sub(1)?;
        entry_tree_with_paths(&self.current_results, 1024)
            .map(|(_, ent)| ent)
            .nth(idx)
    }
    pub fn push_action(&mut self, action: KeyAction) -> ActionResponse {
        match action {
            KeyAction::Up => {
                let nxt = self.selection_position.saturating_sub(1);
                if nxt != self.selection_position {
                    self.selection_position = nxt;
                    ActionResponse::NeedsRedraw
                } else {
                    ActionResponse::Handled
                }
            }
            KeyAction::Down => {
                if self.selection_position >= self.current_results.len() {
                    self.selection_position = 0;
                } else {
                    self.selection_position += 1;
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
                &self.config.font_data,
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
                    &self.config.font_data,
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
