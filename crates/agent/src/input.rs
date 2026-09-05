use std::collections::HashSet;
use std::thread;

use anyhow::{anyhow, Context, Result};
use enigo::{
    Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings,
};
use protocol::{ClickState, KeyState, Message, MouseButton};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub enum InputCmd {
    Message(Message),
    ReleaseAll,
}

pub fn spawn_input_worker(mut rx: mpsc::Receiver<InputCmd>, dry_run: bool) -> Result<()> {
    // Enigo is not Send on some backends; own it on a dedicated OS thread.
    thread::Builder::new()
        .name("input-worker".into())
        .spawn(move || {
            let mut enigo = if dry_run {
                None
            } else {
                match Enigo::new(&Settings::default()) {
                    Ok(e) => Some(e),
                    Err(e) => {
                        error!(?e, "failed to init enigo — falling back to dry-run");
                        None
                    }
                }
            };

            let mut held_keys: HashSet<String> = HashSet::new();
            let mut held_buttons: HashSet<MouseButton> = HashSet::new();

            while let Some(cmd) = rx.blocking_recv() {
                match cmd {
                    InputCmd::Message(msg) => {
                        if let Err(e) = apply(&mut enigo, &msg, &mut held_keys, &mut held_buttons)
                        {
                            warn!(?e, ?msg, "input failed");
                        }
                    }
                    InputCmd::ReleaseAll => {
                        release_all(&mut enigo, &mut held_keys, &mut held_buttons);
                        info!("released all inputs");
                    }
                }
            }
        })
        .context("spawn input worker")?;
    Ok(())
}

fn apply(
    enigo: &mut Option<Enigo>,
    msg: &Message,
    held_keys: &mut HashSet<String>,
    held_buttons: &mut HashSet<MouseButton>,
) -> Result<()> {
    match msg {
        Message::Key { key, state, .. } => {
            let parsed = parse_key(key)?;
            let dir = match state {
                KeyState::Down => {
                    held_keys.insert(key.clone());
                    Direction::Press
                }
                KeyState::Up => {
                    held_keys.remove(key);
                    Direction::Release
                }
            };
            debug!(?key, ?state, "key");
            if let Some(e) = enigo.as_mut() {
                e.key(parsed, dir).map_err(|err| anyhow!("{err:?}"))?;
            }
        }
        Message::MouseMove { dx, dy, .. } => {
            debug!(dx, dy, "mouse_move");
            if let Some(e) = enigo.as_mut() {
                e.move_mouse(*dx, *dy, Coordinate::Rel)
                    .map_err(|err| anyhow!("{err:?}"))?;
            }
        }
        Message::Mouse { button, state, .. } => {
            let btn = to_enigo_button(*button);
            debug!(?button, ?state, "mouse");
            let dir = match state {
                ClickState::Down => {
                    held_buttons.insert(*button);
                    Direction::Press
                }
                ClickState::Up => {
                    held_buttons.remove(button);
                    Direction::Release
                }
                ClickState::Click => Direction::Click,
            };
            if let Some(e) = enigo.as_mut() {
                e.button(btn, dir).map_err(|err| anyhow!("{err:?}"))?;
            }
        }
        Message::Heartbeat { .. } => {}
    }
    Ok(())
}

fn release_all(
    enigo: &mut Option<Enigo>,
    held_keys: &mut HashSet<String>,
    held_buttons: &mut HashSet<MouseButton>,
) {
    if let Some(e) = enigo.as_mut() {
        for key in held_keys.iter() {
            if let Ok(parsed) = parse_key(key) {
                let _ = e.key(parsed, Direction::Release);
            }
        }
        for button in held_buttons.iter() {
            let _ = e.button(to_enigo_button(*button), Direction::Release);
        }
    }
    held_keys.clear();
    held_buttons.clear();
}

fn to_enigo_button(b: MouseButton) -> Button {
    match b {
        MouseButton::Left => Button::Left,
        MouseButton::Right => Button::Right,
        MouseButton::Middle => Button::Middle,
    }
}

fn parse_key(s: &str) -> Result<Key> {
    let lower = s.to_ascii_lowercase();
    Ok(match lower.as_str() {
        "space" => Key::Space,
        "enter" | "return" => Key::Return,
        "escape" | "esc" => Key::Escape,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "shift" => Key::Shift,
        "ctrl" | "control" => Key::Control,
        "alt" => Key::Alt,
        "meta" | "cmd" | "super" | "win" => Key::Meta,
        "up" | "uparrow" => Key::UpArrow,
        "down" | "downarrow" => Key::DownArrow,
        "left" | "leftarrow" => Key::LeftArrow,
        "right" | "rightarrow" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        f if f.starts_with('f') && f[1..].chars().all(|c| c.is_ascii_digit()) => {
            let n: u32 = f[1..].parse().map_err(|_| anyhow!("bad F-key: {s}"))?;
            match n {
                1 => Key::F1,
                2 => Key::F2,
                3 => Key::F3,
                4 => Key::F4,
                5 => Key::F5,
                6 => Key::F6,
                7 => Key::F7,
                8 => Key::F8,
                9 => Key::F9,
                10 => Key::F10,
                11 => Key::F11,
                12 => Key::F12,
                _ => return Err(anyhow!("unsupported F-key: {s}")),
            }
        }
        other => {
            let mut chars = other.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Unicode(c),
                _ => return Err(anyhow!("unknown key: {s}")),
            }
        }
    })
}
