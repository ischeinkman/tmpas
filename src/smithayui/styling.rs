use andrew::shapes::rectangle::Rectangle;
use andrew::text::load_font_file;
use andrew::text::Text;
use anyhow::{anyhow, Context, Error};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use std::panic::catch_unwind;
use std::path::Path;
use std::path::PathBuf;

use crate::model::{EntryPath, ListEntry};

pub type Color = [u8; 4];

pub struct WindowConfig {
    pub dims: (u32, u32),
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { dims: (1024, 576) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FontConfig {
    pub name: Option<String>,
    pub modifiers: FontModifiers,
    pub serif: bool,
    pub mono: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FontModifiers {
    pub bold: bool,
    pub italic: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
pub struct ColorPair {
    pub fg: Color,
    pub bg: Color,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SearchbarConfig {
    pub label_font: FontConfig,
    pub label_size: f32,

    pub buffer_font: FontConfig,
    pub buffer_size: f32,
    pub buffer_inner_padding: usize,

    pub padding: usize,
    pub spacing: usize,

    pub colors: SearchbarColorConfig,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
pub struct SearchbarColorConfig {
    pub label: ColorPair,
    pub buffer: ColorPair,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryListConfig {
    pub font_size: f32,
    pub font_data: FontConfig,
    pub entry_spacing: usize,
    pub colors: EntryListColorConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryListColorConfig {
    pub entries: EntryColorConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryColorConfig {
    pub term: ColorPair,
    pub normal: ColorPair,
    pub term_selected: ColorPair,
    pub normal_selected: ColorPair,
}

impl Default for SearchbarConfig {
    fn default() -> Self {
        SearchbarConfig {
            label_font: FontConfig {
                serif: false,
                mono: false,
                modifiers: FontModifiers {
                    bold: true,
                    italic: false,
                },
                name: None,
            },
            label_size: 32.0,
            buffer_font: FontConfig::default(),
            buffer_size: 32.0,
            buffer_inner_padding: 1,
            padding: 8,
            spacing: 16,
            colors: SearchbarColorConfig {
                buffer: ColorPair {
                    fg: [255, 0, 0, 0],
                    bg: [255, 255, 255, 0],
                },
                label: ColorPair {
                    fg: [255, 0, 0, 0],
                    bg: [255, 0, 0, 0],
                },
            },
        }
    }
}
impl Default for EntryListConfig {
    fn default() -> EntryListConfig {
        EntryListConfig {
            font_size: 32.0,
            font_data: Default::default(),
            entry_spacing: 8,
            colors: EntryListColorConfig {
                entries: EntryColorConfig {
                    term: ColorPair {
                        fg: [0xFF, 0x1d, 0x7f, 0x25],
                        bg: [0xFF, 0xFF, 0xFF, 0xFF],
                    },
                    term_selected: ColorPair {
                        fg: [0xFF, 0xff, 0xff, 0xff],
                        bg: [0xF0, 0x00, 0x3E, 0x1C],
                    },
                    normal: ColorPair {
                        fg: [0xFF, 0x00, 0x00, 0x00],
                        bg: [0xF0, 0xFF, 0xFF, 0xFF],
                    },
                    normal_selected: ColorPair {
                        fg: [0xFF, 0xff, 0xff, 0xff],
                        bg: [0xF0, 0x00, 0x10, 0x08],
                    },
                },
            },
        }
    }
}

impl EntryListConfig {
    pub fn new() -> Result<Self, Error> {
        Ok(Self::default())
    }
    pub fn text_color(&self, _path: EntryPath, entry: &ListEntry, selected: bool) -> [u8; 4] {
        let is_term = entry.exec_flags.is_term();
        match (selected, is_term) {
            (true, true) => self.colors.entries.term_selected.fg,
            (false, true) => self.colors.entries.term.fg,
            (true, false) => self.colors.entries.normal_selected.fg,
            (false, false) => self.colors.entries.normal.fg,
        }
    }
    pub fn background_color(&self, _path: EntryPath, entry: &ListEntry, selected: bool) -> [u8; 4] {
        let is_term = entry.exec_flags.is_term();
        match (selected, is_term) {
            (true, true) => self.colors.entries.term_selected.bg,
            (false, true) => self.colors.entries.term.bg,
            (true, false) => self.colors.entries.normal_selected.bg,
            (false, false) => self.colors.entries.normal.bg,
        }
    }
    pub fn entry_height(&self) -> usize {
        self.font_size.ceil() as usize + self.entry_spacing
    }
    pub fn prefix_size(&self, level: usize) -> usize {
        level * (self.font_size as usize)
    }
}

impl SearchbarConfig {
    pub fn label_text(&self) -> Text<'_> {
        let x = self.padding;
        let y = self.padding + (self.inner_height() - self.label_size as usize) / 2;

        Text::new(
            (x, y),
            self.colors.label.fg,
            self.label_font.get_font().unwrap(),
            self.label_size,
            1.0,
            "Search: ",
        )
    }
    pub fn buffer_background(&self, label_width: usize, canvas_width: usize) -> Rectangle {
        let x = self.buffer_rect_x(label_width);
        let y = self.buffer_rect_y();
        let w = self.buffer_rect_width(label_width, canvas_width);
        let h = self.buffer_rect_height();
        Rectangle::new((x, y), (w, h), None, Some(self.colors.buffer.bg))
    }
    pub fn buffer_text<'a>(&'a self, label_width: usize, buffer: &str) -> Text {
        let x = self.buffer_rect_x(label_width) + self.buffer_inner_padding;
        let y = self.padding + (self.inner_height() - self.buffer_size as usize) / 2;

        Text::new(
            (x, y),
            self.colors.label.fg,
            self.buffer_font.get_font().unwrap(),
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
}
impl FontConfig {
    pub fn get_font<'a>(&self) -> Result<&'a [u8], Error> {
        FONT_STORE.get(&self)
    }
    fn load_font(&self) -> Result<Vec<u8>, Error> {
        let fontpath = self.find_font()?;
        eprintln!("Loading font {:?} for params {:?}.", fontpath, self);
        let font_data = catch_unwind(|| load_font_file(&fontpath))
            .map_err(|e| {
                e.downcast::<String>()
                    .map(Error::msg)
                    .or_else(|e| e.downcast::<&str>().map(Error::msg))
                    .unwrap_or_else(|_| Error::msg("Unknown panic occurred."))
            })
            .with_context(|| {
                format!(
                    "Error reading font path {} as font data.",
                    fontpath.display()
                )
            })?;
        Ok(font_data)
    }
    fn find_font(&self) -> Result<PathBuf, Error> {
        let font_config = andrew::text::fontconfig::FontConfig::new()
            .map_err(|_| anyhow!("Could not construct FontConfig."))?;
        let all_fonts = font_config
            .get_fonts()
            .with_context(|| "Error getting font list from config.")?;
        all_fonts
            .into_iter()
            .find(|fnt| self.matches(fnt))
            .ok_or_else(|| {
                anyhow::anyhow!("Could not find font matching requirements : {:?}", self)
            })
    }
    fn matches(&self, path: &Path) -> bool {
        let fname = match path.file_name() {
            Some(f) => f,
            None => {
                return false;
            }
        };
        let fname = fname.to_string_lossy();
        if let Some(name) = self.name.as_deref() {
            if !fname.to_lowercase().contains(name) {
                return false;
            }
        }
        let is_bold = fname.contains("Bold") || fname.contains("_bold") || fname.contains("-bold");
        if is_bold != self.modifiers.bold {
            return false;
        }
        let is_italic =
            fname.contains("Italic") || fname.contains("_italic") || fname.contains("-italic");
        let is_oblique =
            fname.contains("Oblique") || fname.contains("_oblique") || fname.contains("-oblique");
        if (is_italic || is_oblique) != self.modifiers.italic {
            return false;
        }
        let is_mono = fname.contains("Mono") || fname.contains("_mono") || fname.contains("-mono");
        if is_mono != self.mono {
            return false;
        }
        let is_sans = fname.contains("Sans") || fname.contains("_sans") || fname.contains("-sans");
        if !is_sans != self.serif {
            return false;
        }
        true
    }
}

static FONT_STORE: FontStore = FontStore::new();
struct FontStore {
    cache: [OnceCell<(FontConfig, Vec<u8>)>; 32],
}
impl FontStore {
    const fn new() -> Self {
        #[allow(clippy::clippy::declare_interior_mutable_const)]
        const _INNER: OnceCell<(FontConfig, Vec<u8>)> = OnceCell::new();
        Self {
            cache: [_INNER; 32],
        }
    }
    fn get(&self, font: &FontConfig) -> Result<&[u8], Error> {
        let mut slots = self.cache.iter().filter_map(|slot| slot.get());
        let existing = slots.find(|(k, _)| k == font).map(|(_, v)| v);
        if let Some(existing) = existing {
            return Ok(&existing);
        }
        let font_data = font.load_font()?;
        loop {
            let next_slot = self.cache.iter().find(|slot| slot.get().is_none());
            if let Some(slot) = next_slot {
                let inserted = slot.get_or_init(|| (font.clone(), font_data.clone()));
                if &inserted.0 == font {
                    return Ok(&inserted.1);
                }
            } else {
                return Err(anyhow!("Font slots have been filled."));
            }
        }
    }
}
