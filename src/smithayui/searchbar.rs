use super::ActionResponse;
use super::KeyAction;
use super::Rect;

use andrew::shapes::rectangle::Rectangle;
use andrew::text::Text;
use andrew::Canvas;

use std::cmp::{Ord, PartialOrd};
use std::ops::Sub;

fn abs_sub<O, N: PartialOrd + Sub<Output = O>>(a: N, b: N) -> O {
    if a > b {
        a - b
    } else {
        b - a
    }
}

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
        let previous_color = [
            output.buffer[borders.y * output.stride + borders.x * output.pixel_size],
            output.buffer[borders.y * output.stride + borders.x * output.pixel_size + 1],
            output.buffer[borders.y * output.stride + borders.x * output.pixel_size + 2],
            output.buffer[borders.y * output.stride + borders.x * output.pixel_size + 3],
        ];
        output.draw(&label);
        for y in 0..(self.config.label_size as usize) {
            for x in 0..label.get_width() {
                let x = label.pos.0 + x;
                let y = label.pos.1 + y;
                let idx = y * output.stride + x * output.pixel_size;
                let color = &mut output.buffer[idx - 1..idx - 1 + output.pixel_size];
                let d_label: u16 = color
                    .iter()
                    .zip(self.config.label_color.iter())
                    .map(|(a, b)| abs_sub(a, b))
                    .fold(0, |a, b| a + (b as u16));
                let d_prev: u16 = color
                    .iter()
                    .zip(previous_color.iter())
                    .map(|(a, b)| abs_sub(a, b))
                    .fold(0, |a, b| a + (b as u16));
                if d_prev < d_label {
                    color.copy_from_slice(&previous_color);
                }
            }
        }
        output.draw(&buffer_rect);
        output.draw(&buffer_text);
    }
}

pub struct SearchbarConfig {
    pub label_font: Vec<u8>,
    pub label_size: f32,

    pub buffer_font: Vec<u8>,
    pub buffer_size: f32,
    pub buffer_inner_padding: usize,

    pub padding: usize,
    pub spacing: usize,

    pub label_color: [u8; 4],

    pub buffer_color: [u8; 4],
    pub buffer_background: [u8; 4],
}

impl SearchbarConfig {
    fn label_text(&self) -> Text {
        let x = self.label_x();
        let y = self.label_y();
        Text::new(
            (x, y),
            self.label_color,
            &self.label_font,
            self.label_size,
            1.0,
            "Search: ",
        )
    }
    fn buffer_background(&self, label_width: usize, canvas_width: usize) -> Rectangle {
        let x = self.buffer_rect_x(label_width);
        let y = self.buffer_rect_y();
        let w = self.buffer_rect_width(label_width, canvas_width);
        let h = self.buffer_rect_height();
        Rectangle::new((x, y), (w, h), None, Some(self.buffer_background))
    }
    fn buffer_text(&self, label_width: usize, buffer: &str) -> Text {
        let x = self.buffer_text_x(label_width);
        let y = self.buffer_text_y();

        Text::new(
            (x, y),
            self.label_color,
            &self.buffer_font,
            self.buffer_size,
            1.0,
            buffer,
        )
    }
    pub const fn outer_height(&self) -> usize {
        self.inner_height() + 2 * self.padding
    }
    const fn inner_height(&self) -> usize {
        let label_height = self.label_size as usize;
        let buffer_height = self.buffer_rect_height();
        if label_height > buffer_height {
            label_height
        } else {
            buffer_height
        }
    }
    const fn label_x(&self) -> usize {
        self.padding
    }
    const fn label_y(&self) -> usize {
        self.padding + (self.inner_height() - self.label_size as usize) / 2
    }
    const fn buffer_rect_x(&self, label_width: usize) -> usize {
        self.padding + label_width + self.spacing
    }
    const fn buffer_rect_y(&self) -> usize {
        self.padding
            + (self.inner_height() - self.buffer_size as usize - self.buffer_inner_padding) / 2
    }
    const fn buffer_rect_width(&self, label_width: usize, canvas_width: usize) -> usize {
        let no_start = canvas_width.saturating_sub(self.buffer_rect_x(label_width));
        no_start.saturating_sub(self.padding)
    }
    const fn buffer_rect_height(&self) -> usize {
        self.buffer_size as usize + self.buffer_inner_padding * 2
    }
    const fn buffer_text_x(&self, label_width: usize) -> usize {
        self.buffer_rect_x(label_width) + self.buffer_inner_padding
    }
    const fn buffer_text_y(&self) -> usize {
        self.padding + (self.inner_height() - self.buffer_size as usize) / 2
    }
}
