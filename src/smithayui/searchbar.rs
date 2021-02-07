use super::{ActionResponse,};
use super::KeyAction;
use super::Rect;

use andrew::Canvas;
use super::styling::SearchbarConfig;
use std::cmp::Ord;

pub struct SearchBar {
    pub config: SearchbarConfig,
    pub buffer: String,
    pub cursor: usize,
}

impl SearchBar {
    pub fn new(config: SearchbarConfig) -> Self {
        Self {
            config,
            buffer: String::new(),
            cursor: 0,
        }
    }
    pub fn push_action(&mut self, action: KeyAction) -> ActionResponse {
        match action {
            KeyAction::Backspace => {
                if self.cursor > 0 {
                    let cur_len = self.buffer.len();
                    let mut prev_iter = (&self.buffer[..self.cursor])
                        .char_indices()
                        .rev()
                        .map(|(idx, _)| idx);
                    let to_remove = prev_iter.find(|idx| *idx != self.cursor);
                    if let Some(to_remove) = to_remove {
                        self.buffer.remove(to_remove);
                        let new_len = self.buffer.len();
                        self.cursor = (self.cursor + new_len).saturating_sub(cur_len);
                        return ActionResponse::NeedsRedraw;
                    }
                }
                ActionResponse::Handled
            }
            KeyAction::Character(c) => {
                if self.cursor == self.buffer.len() {
                    self.buffer.push_str(&c);
                } else {
                    self.buffer.insert_str(self.cursor, &c);
                }
                self.cursor += c.len();
                ActionResponse::NeedsRedraw
            }
            KeyAction::Left => {
                if let Some(nxt) = self.cursor.checked_sub(1) {
                    self.cursor = nxt;
                    ActionResponse::NeedsRedraw
                } else {
                    ActionResponse::Handled
                }
            }
            KeyAction::Right => {
                let nxt = self.cursor.saturating_add(1).min(self.buffer.len());
                if nxt != self.cursor {
                    self.cursor = nxt;
                    ActionResponse::NeedsRedraw
                } else {
                    ActionResponse::Handled
                }
            }
            other => ActionResponse::Continue(other),
        }
    }
    pub fn display(&mut self, borders: Rect, output: &mut Canvas) {
        let mut label = self.config.label_text();
        label.pos.0 += borders.x;
        label.pos.1 += borders.y;

        let mut buffer_rect = self
            .config
            .buffer_background(label.get_width(), borders.width);
        buffer_rect.pos.0 += borders.x;
        buffer_rect.pos.1 += borders.y;
        let mut buffer_text = self.config.buffer_text(label.get_width(), &self.buffer);
        buffer_text.pos.0 += borders.x;
        buffer_text.pos.1 += borders.y;
        output.draw(&label);
        output.draw(&buffer_rect);
        output.draw(&buffer_text);
    }
}
