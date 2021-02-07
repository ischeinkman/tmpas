use crate::{model::ListEntry, State};

use smithay_client_toolkit as sctk;

use sctk::reexports::calloop;
use sctk::seat::keyboard::keysyms;
use sctk::seat::keyboard::KeyState;
use sctk::seat::keyboard::{map_keyboard_repeat, Event as KbEvent, RepeatKind};
use sctk::shm::MemPool;
use sctk::window::{ConceptFrame, Event as WEvent};
use wayland_client::protocol::{wl_keyboard, wl_shm, wl_surface};

use std::io::{self, Seek, SeekFrom, Write};

mod resultslist;
use resultslist::EntryList;
mod searchbar;
use searchbar::SearchBar;
mod styling;
use styling::{EntryListConfig, SearchbarConfig, WindowConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionResponse {
    NeedsRedraw,
    Handled,
    Continue(KeyAction),
}

sctk::default_environment!(SmithayUi, desktop);

#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub enum KeyAction {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Enter,
    Backspace,
    Character(String),
}

impl KeyAction {
    pub fn from_event(event: KbEvent) -> Option<Self> {
        let (keysym, buff) = match event {
            KbEvent::Key {
                keysym,
                utf8,
                state: KeyState::Pressed,
                ..
            }
            | KbEvent::Repeat { keysym, utf8, .. } => (keysym, utf8),
            _ => {
                return None;
            }
        };
        Self::from_keysym(keysym).or_else(|| buff.map(KeyAction::Character))
    }
    pub fn from_keysym(keysym: u32) -> Option<Self> {
        match keysym {
            keysyms::XKB_KEY_KP_Up | keysyms::XKB_KEY_Up => Some(KeyAction::Up),
            keysyms::XKB_KEY_KP_Down | keysyms::XKB_KEY_Down => Some(KeyAction::Down),
            keysyms::XKB_KEY_KP_Left | keysyms::XKB_KEY_Left => Some(KeyAction::Left),
            keysyms::XKB_KEY_KP_Right | keysyms::XKB_KEY_Right => Some(KeyAction::Right),
            keysyms::XKB_KEY_KP_Page_Up | keysyms::XKB_KEY_Page_Up => Some(KeyAction::PageUp),
            keysyms::XKB_KEY_KP_Page_Down | keysyms::XKB_KEY_Page_Down => Some(KeyAction::PageDown),
            keysyms::XKB_KEY_Return
            | keysyms::XKB_KEY_KP_Enter
            | keysyms::XKB_KEY_ISO_Enter
            | keysyms::XKB_KEY_3270_Enter => Some(KeyAction::Enter),
            keysyms::XKB_KEY_BackSpace | keysyms::XKB_KEY_osfBackSpace => {
                Some(KeyAction::Backspace)
            }

            _ => None,
        }
    }
}

pub struct EventStore {
    window_event: Option<WEvent>,
    key_events: Vec<KeyAction>,
}

impl EventStore {
    pub fn new() -> Self {
        Self {
            window_event: None,
            key_events: Vec::with_capacity(16),
        }
    }
}
pub fn run(state: State) {
    if let Some((state, to_run)) = run_inner(state) {
        state.run(&to_run);
    }
}

fn run_inner(mut state: State) -> Option<(State, ListEntry)> {
    /*
     * Initial setup
     */
    let (env, display, queue) =
        sctk::new_default_environment!(SmithayUi, desktop).expect("Could not open compositor");

    /*
     * Prepare a calloop event loop to handle key repetion
     */
    // Here `Option<WEvent>` is the type of a global value that will be shared by
    // all callbacks invoked by the event loop.
    let mut event_loop = calloop::EventLoop::<EventStore>::new().unwrap();

    /*
     * Keyboard initialization
     */
    //==================================================
    let mut seats = Vec::<(
        String,
        Option<(wl_keyboard::WlKeyboard, calloop::Source<_>)>,
    )>::new();

    // first process already existing seats
    for seat in env.get_all_seats() {
        let seat_data = sctk::seat::with_seat_data(&seat, |seat_data| {
            (
                seat_data.has_keyboard && !seat_data.defunct,
                seat_data.name.clone(),
            )
        });
        if let Some((has_kbd, name)) = seat_data {
            if !has_kbd {
                seats.push((name, None));
                continue;
            }
            match map_keyboard_repeat(
                event_loop.handle(),
                &seat,
                None,
                RepeatKind::System,
                move |event, _, mut dd| {
                    if let Some(act) = KeyAction::from_event(event) {
                        let store = dd.get::<EventStore>().unwrap();
                        store.key_events.push(act);
                    }
                },
            ) {
                Ok((kbd, repeat_source)) => {
                    seats.push((name, Some((kbd, repeat_source))));
                }
                Err(e) => {
                    eprintln!("Failed to map keyboard on seat {} : {:?}.", name, e);
                    seats.push((name, None));
                }
            }
        }
    }

    // then setup a listener for changes
    let loop_handle = event_loop.handle();
    let _seat_listener = env.listen_for_seats(move |seat, seat_data, _| {
        // find the seat in the vec of seats, or insert it if it is unknown
        let idx = seats.iter().position(|(name, _)| name == &seat_data.name);
        let idx = idx.unwrap_or_else(|| {
            seats.push((seat_data.name.clone(), None));
            seats.len() - 1
        });

        let (_, ref mut opt_kbd) = &mut seats[idx];
        // we should map a keyboard if the seat has the capability & is not defunct
        if seat_data.has_keyboard && !seat_data.defunct {
            if opt_kbd.is_some() {
                return;
            }
            // we should initalize a keyboard
            match map_keyboard_repeat(
                loop_handle.clone(),
                &seat,
                None,
                RepeatKind::System,
                move |event, _, mut dd| {
                    if let Some(act) = KeyAction::from_event(event) {
                        let store = dd.get::<EventStore>().unwrap();
                        store.key_events.push(act);
                    }
                },
            ) {
                Ok((kbd, repeat_source)) => {
                    *opt_kbd = Some((kbd, repeat_source));
                }
                Err(e) => {
                    eprintln!(
                        "Failed to map keyboard on seat {} : {:?}.",
                        seat_data.name, e
                    )
                }
            }
        } else if let Some((kbd, source)) = opt_kbd.take() {
            // the keyboard has been removed, cleanup
            kbd.release();
            loop_handle.remove(source);
        }
    });
    //==================================================
    let surface = env.create_surface().detach();
    let cfg = WindowConfig::default();
    let mut dimensions = cfg.dims;
    let mut window = env
        .create_window::<ConceptFrame, _>(
            surface,
            None,
            dimensions,
            move |evt, mut dispatch_data| {
                let store = dispatch_data.get::<EventStore>().unwrap();
                let next_action = &mut store.window_event;
                // Keep last event in priority order : Close > Configure > Refresh
                let replace = matches!((&evt, &*next_action), (_, &None)
                    | (_, &Some(WEvent::Refresh))
                    | (&WEvent::Configure { .. }, &Some(WEvent::Configure { .. }))
                    | (&WEvent::Close, _));
                if replace {
                    *next_action = Some(evt);
                }
            },
        )
        .expect("Failed to create a window !");

    window.set_title("TMPAS".to_string());
    window.set_app_id("tmpas".to_string());
    window.set_resizable(false);
    window.set_decorate(sctk::window::Decorations::ClientSide);
    let mut pools = env
        .create_double_pool(|_| {})
        .expect("Failed to create a memory pool !");
    //==================================================
    let bar_cfg = SearchbarConfig::default();
    let mut bar = SearchBar::new(bar_cfg);
    let mut resl = EntryList::new(EntryListConfig::new().unwrap());
    resl.set_results(state.search("", 4 * resl.max_entries()));
    if !env.get_shell().unwrap().needs_configure() {
        // initial draw to bootstrap on wl_shell
        if let Some(pool) = pools.pool() {
            redraw(&mut bar, &mut resl, pool, window.surface(), dimensions).expect("Failed to draw")
        }
        window.refresh();
    }

    let mut next_action = EventStore::new();

    sctk::WaylandSource::new(queue)
        .quick_insert(event_loop.handle())
        .unwrap();

    let mut needs_redraw = false;
    let mut can_expand = true;
    loop {
        let mut had_handled = false;
        let old_buffer = bar.buffer.clone();
        for action in next_action.key_events.drain(..) {
            if action == KeyAction::Enter {
                if let Some(selected) = resl.selected().cloned() {
                    return Some((state, selected));
                }
            }
            let action = match bar.push_action(action) {
                ActionResponse::NeedsRedraw => {
                    needs_redraw = true;
                    had_handled = true;
                    continue;
                }
                ActionResponse::Handled => {
                    had_handled = true;
                    continue;
                }
                ActionResponse::Continue(action) => action,
            };
            let _action = match resl.push_action(action) {
                ActionResponse::NeedsRedraw => {
                    needs_redraw = true;
                    had_handled = true;
                    continue;
                }
                ActionResponse::Handled => {
                    had_handled = true;
                    continue;
                }
                ActionResponse::Continue(action) => action,
            };
        }
        if old_buffer != bar.buffer {
            resl.set_results(state.search(&bar.buffer, 4 * resl.max_entries()));
            needs_redraw = true;
            can_expand = true;
        } else if can_expand && resl.buffer_height() <= resl.max_entries() / 2 {
            let target_height = resl.cur_results_height() + resl.max_entries() * 2;
            let new_buffer = state.search(&bar.buffer, target_height);
            resl.set_buffer(new_buffer);
            can_expand = resl.cur_results_height() >= target_height;
            needs_redraw = true;
        }
        if had_handled {
            eprintln!(
                "Finished key events. Buffer: {:?}, {:?}",
                bar.buffer, bar.cursor
            );
        }
        match next_action.window_event.take() {
            Some(WEvent::Close) => {
                return None;
            }
            Some(WEvent::Refresh) => {
                window.refresh();
                window.surface().commit();
            }
            Some(WEvent::Configure { new_size, states }) => {
                if let Some((w, h)) = new_size {
                    window.resize(w, h);
                    dimensions = (w, h)
                }
                println!("Window states: {:?}", states);
                needs_redraw = true;
            }
            None => {}
        }
        if needs_redraw {
            window.refresh();
            if let Some(pool) = pools.pool() {
                eprintln!("Doing redraw.");
                redraw(&mut bar, &mut resl, pool, window.surface(), dimensions)
                    .expect("Failed to draw");
                needs_redraw = false;
            }
        }

        // always flush the connection before going to sleep waiting for events
        display.flush().unwrap();
        event_loop.dispatch(None, &mut next_action).unwrap();
    }
}
fn redraw(
    obj: &mut SearchBar,
    resl: &mut resultslist::EntryList,
    pool: &mut MemPool,
    surface: &wl_surface::WlSurface,
    (buf_x, buf_y): (u32, u32),
) -> Result<(), io::Error> {
    let buf_x = buf_x as usize;
    let buf_y = buf_y as usize;
    pool.resize(4 * buf_x * buf_y)
        .expect("Failed to resize the memory pool.");
    pool.seek(SeekFrom::Start(0))?;
    let pool_mem = pool.mmap();
    for idx in 0..4 * buf_x * buf_y {
        pool_mem[idx] = 255;
    }
    let mut canvas =
        andrew::Canvas::new(pool_mem, buf_x, buf_y, 4 * buf_x, andrew::Endian::native());
    obj.display(
        Rect {
            x: 0,
            y: 0,
            width: canvas.width,
            height: canvas.height,
        },
        &mut canvas,
    );
    let list_y = obj.config.outer_height() + obj.config.padding;
    let list_height = canvas.height - list_y;
    resl.display(
        Rect {
            x: 0,
            y: list_y,
            width: canvas.width,
            height: list_height,
        },
        &mut canvas,
    );
    pool.flush()?;

    let new_buffer = pool.buffer(
        0,
        buf_x as i32,
        buf_y as i32,
        4 * buf_x as i32,
        wl_shm::Format::Argb8888,
    );
    surface.attach(Some(&new_buffer), 0, 0);
    // damage the surface so that the compositor knows it needs to redraw it
    if surface.as_ref().version() >= 4 {
        // If our server is recent enough and supports at least version 4 of the
        // wl_surface interface, we can specify the damage in buffer coordinates.
        // This is obviously the best and do that if possible.
        surface.damage_buffer(0, 0, buf_x as i32, buf_y as i32);
    } else {
        // Otherwise, we fallback to compatilibity mode. Here we specify damage
        // in surface coordinates, which would have been different if we had drawn
        // our buffer at HiDPI resolution. We didn't though, so it is ok.
        // Using `damage_buffer` in general is better though.
        surface.damage(0, 0, buf_x as i32, buf_y as i32);
    }

    surface.commit();
    Ok(())
}
