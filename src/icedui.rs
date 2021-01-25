use crate::model::{entry_tree_get, entry_tree_with_paths, EntryPath, ListEntry};
use crate::{AppMessage, State};

use iced::window;
use iced::{container, Background};
use iced::{
    widget::{text_input, Column, Container, Row, Text, TextInput},
    Color,
};
use iced::{
    Align, Application, Command, Element, HorizontalAlignment, Length, Settings, Subscription,
    VerticalAlignment,
};
use iced_futures::executor::Executor;
use iced_native::keyboard::Event as KeyboardEvent;
use iced_native::keyboard::KeyCode;
use iced_native::Event;

use futures::FutureExt;

use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc;
use std::task::Poll;
use std::thread;

pub fn run(state: State) {
    let mut settings = Settings::with_flags(state);
    settings.window = window::Settings {
        always_on_top: true,
        decorations: true,
        resizable: false,
        transparent: true,

        ..Default::default()
    };
    settings.antialiasing = true;
    IcedUi::run(settings).unwrap();
}

pub struct IcedUiExecutor {
    _background_handle: thread::JoinHandle<()>,
    sender: mpsc::SyncSender<Box<dyn std::future::Future<Output = ()> + Send + 'static>>,
}

impl Executor for IcedUiExecutor {
    fn new() -> Result<Self, futures::io::Error> {
        let (sender, recv) = mpsc::sync_channel(8);
        let mut task_queue = Vec::new();
        let _background_handle = thread::spawn(move || loop {
            match recv.try_recv() {
                Ok(fut) => task_queue.push(Pin::from(fut)),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    break;
                }
            }
            if task_queue.is_empty() {
                match recv.recv() {
                    Ok(vl) => {
                        task_queue.push(Pin::from(vl));
                    }
                    Err(_) => {
                        break;
                    }
                };
            }
            let mut next_queue = Vec::new();
            for fut in task_queue.drain(..) {
                let mut fut: Pin<Box<dyn Future<Output = ()> + Send + 'static>> = fut;
                let waker = futures::task::noop_waker();
                let mut cx = futures::task::Context::from_waker(&waker);
                let out: Poll<()> = fut.poll_unpin(&mut cx);
                if out.is_pending() {
                    next_queue.push(fut);
                }
            }
            task_queue = next_queue;
        });
        Ok(Self {
            sender,
            _background_handle,
        })
    }
    fn spawn(&self, future: impl std::future::Future<Output = ()> + Send + 'static) {
        self.sender
            .send(Box::new(future))
            .expect("Background executor died.");
    }
    fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
        f()
    }
}
#[derive(Debug, Clone)]
pub enum Message {
    #[allow(unused)]
    Backend(AppMessage),
    SetBuffer(String),
    CursorUp,
    CursorDown,
    RunSelected,
}

pub struct IcedUi {
    app_state: super::State,
    search_buffer: SearchBuffer,
    entry_list: EntryList,
}

impl Application for IcedUi {
    type Executor = IcedUiExecutor;
    type Message = Message;
    type Flags = super::State;

    fn new(app_state: Self::Flags) -> (Self, Command<Self::Message>) {
        let search_buffer = SearchBuffer::new();
        let mut entry_list = EntryList::new();
        let entries = app_state.all_entries();
        entry_list.set_results(entries);
        let res = Self {
            app_state,
            search_buffer,
            entry_list,
        };
        (res, Command::none())
    }
    fn title(&self) -> String {
        "TMPAS Application Runner".into()
    }
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Backend(AppMessage::SearchResults(results)) => {
                self.entry_list.set_results(results);
                Command::none()
            }
            Message::SetBuffer(buf) => {
                self.search_buffer.buffer = buf;
                let new_res = self.app_state.search_loaded(&self.search_buffer.buffer);
                self.entry_list.set_results(new_res);
                Command::none()
            }
            Message::CursorUp => {
                self.entry_list.cursor_up();
                Command::none()
            }
            Message::CursorDown => {
                self.entry_list.cursor_down();
                Command::none()
            }
            Message::RunSelected => {
                if let Some(ent) = self.entry_list.selected() {
                    self.app_state.run(ent);
                }
                Command::none()
            }
            #[allow(unreachable_patterns)]
            _ => {
                unreachable!()
            }
        }
    }

    fn mode(&self) -> window::Mode {
        window::Mode::Windowed
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let elm = Column::new()
            .push(self.search_buffer.display())
            .push(self.entry_list.display())
            .align_items(Align::Start);
        let elm = Container::new(elm).style(StyleWrapper(container::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        }));
        elm.into()
    }
    fn subscription(&self) -> Subscription<Self::Message> {
        iced_native::subscription::events_with(|evt, _| match evt {
            Event::Keyboard(KeyboardEvent::KeyPressed {
                key_code: KeyCode::Up,
                ..
            }) => Some(Message::CursorUp),
            Event::Keyboard(KeyboardEvent::KeyPressed {
                key_code: KeyCode::Down,
                ..
            }) => Some(Message::CursorDown),
            Event::Keyboard(KeyboardEvent::KeyPressed {
                key_code: KeyCode::Enter,
                ..
            }) => Some(Message::RunSelected),
            _ => None,
        })
    }
}

#[derive(Debug)]
pub struct EntryList {
    current_results: Vec<ListEntry>,
    selected: EntryPath,
    view_offset: usize,
    view_length: usize,
}

const MAX_EXPANSION: usize = 1000;

impl EntryList {
    pub fn new() -> Self {
        Self {
            current_results: Vec::new(),
            selected: EntryPath::new().then(0),
            view_offset: 0,
            view_length: 30,
        }
    }
    pub fn set_results(&mut self, new_results: Vec<ListEntry>) {
        self.current_results = new_results;
        self.selected = EntryPath::new();
        self.view_offset = 0;
    }

    pub fn cursor_up(&mut self) {
        if let Some(nxt) = self.selected.prev_sibling() {
            let mut sibling_ent = entry_tree_get(&self.current_results, nxt).unwrap();
            let mut next_path = nxt;
            while let Some((idx, ent)) = sibling_ent.children.iter().enumerate().last() {
                next_path = next_path.then(idx);
                sibling_ent = ent;
            }
            self.selected = next_path;
        }
        else {
            self.selected = self.selected.parent();
        }
        self.correct_offset();
    }

    pub fn cursor_down(&mut self) {
        let mut cur_next = self.selected.then(0);
        loop {
            let cur_next_ent = entry_tree_get(&self.current_results, cur_next);
            if cur_next_ent.is_some() {
                break;
            }
            match cur_next.parent().next_sibling() {
                Some(n) => {
                    cur_next = n;
                }
                None => {
                    break;
                }
            }
        }
        self.selected = cur_next;
        self.correct_offset();
    }

    fn correct_offset(&mut self) {
        let selection_idx = entry_tree_with_paths(&self.current_results, 1024)
            .map(|(path, _)| path)
            .enumerate()
            .find(|(_, pt)| *pt == self.selected)
            .map(|(idx, _)| idx);
        let selection_idx = match selection_idx {
            Some(n) => n,
            None => {
                return;
            }
        };
        let view_start = self.view_offset;
        let view_end = self.view_offset + self.view_length;
        if selection_idx >= view_end {
            self.view_offset = selection_idx - self.view_length + 1;
        } else if selection_idx < view_start {
            self.view_offset = selection_idx;
        }
    }

    pub fn selected(&self) -> Option<&ListEntry> {
        entry_tree_get(&self.current_results, self.selected)
    }

    pub fn display(&mut self) -> Element<'_, <IcedUi as Application>::Message> {
        let mut retvl = Column::new();
        let relevant = entry_tree_with_paths(&self.current_results, MAX_EXPANSION)
            .skip(self.view_offset)
            .take(self.view_length);
        for (path, ent) in relevant {
            let level = path.level() - 1;
            let selected = self.selected == path;
            let row = make_child_row(ent, level, selected);
            retvl = retvl.push(row);
        }
        retvl.into()
    }
}

fn entry_row_style(ent: &ListEntry, selected: bool) -> impl container::StyleSheet {
    let base_background = Color::from_rgba8(255, 255, 255, 0.0);
    let base_text = if ent.exec_name().is_none() {
        Color::from_rgb(0.7, 0.7, 0.7)
    } else if ent.exec_flags.is_term() {
        Color::from_rgb(0.9, 0.3, 0.4)
    } else {
        Color::from_rgb(0.4, 0.9, 0.4)
    };
    let (text, background) = if !selected {
        (base_text, base_background)
    } else {
        (base_background, base_text)
    };
    let res = container::Style {
        text_color: Some(text),
        background: Some(Background::Color(background)),
        ..Default::default()
    };
    StyleWrapper(res)
}

struct StyleWrapper(container::Style);
impl container::StyleSheet for StyleWrapper {
    fn style(&self) -> container::Style {
        self.0
    }
}

fn make_child_row(
    ent: &ListEntry,
    level: usize,
    selected: bool,
) -> impl Into<Element<'_, Message>> {
    let retvl = Row::new().width(Length::Fill);
    let prefix = match level {
        0 => Cow::Borrowed(""),
        1 => Cow::Borrowed("|-"),
        _ => {
            let mut prefix = "  ".repeat(level - 2);
            prefix.push_str("|-");
            Cow::Owned(prefix)
        }
    };
    let label = Text::new(format!("{}{}", prefix, ent.name()))
        .width(Length::Fill)
        .height(Length::Units(20))
        .horizontal_alignment(HorizontalAlignment::Left)
        .vertical_alignment(VerticalAlignment::Center);
    let retvl = retvl.push(label);

    let style = entry_row_style(ent, selected);
    Container::new(retvl).style(style)
}

#[derive(Debug, Default)]
pub struct SearchBuffer {
    pub state: text_input::State,
    pub buffer: String,
    pub cursor_position: usize,
}

impl SearchBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn display(&mut self) -> Element<'_, <IcedUi as Application>::Message> {
        self.state.focus();
        let input_buffer = TextInput::new(&mut self.state, "", &self.buffer, Message::SetBuffer)
            .width(Length::Fill)
            .padding(5);
        let prompt = Text::new("Search: ").width(Length::Shrink);
        let raw = Row::new()
            .width(Length::Fill)
            .height(Length::Shrink)
            .push(prompt)
            .push(input_buffer)
            .spacing(12);
        Container::new(raw).padding(16).into()
    }
}
