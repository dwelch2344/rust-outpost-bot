use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KeyState {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClickState {
    Down,
    Up,
    Click,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Key { t: f64, key: String, state: KeyState },
    MouseMove { t: f64, dx: i32, dy: i32 },
    Mouse { t: f64, button: MouseButton, state: ClickState },
    Heartbeat { t: f64 },
}

impl Message {
    pub fn key(key: impl Into<String>, state: KeyState) -> Self {
        Self::Key { t: now(), key: key.into(), state }
    }

    pub fn mouse_move(dx: i32, dy: i32) -> Self {
        Self::MouseMove { t: now(), dx, dy }
    }

    pub fn mouse(button: MouseButton, state: ClickState) -> Self {
        Self::Mouse { t: now(), button, state }
    }

    pub fn heartbeat() -> Self {
        Self::Heartbeat { t: now() }
    }

    pub fn timestamp(&self) -> f64 {
        match self {
            Self::Key { t, .. }
            | Self::MouseMove { t, .. }
            | Self::Mouse { t, .. }
            | Self::Heartbeat { t } => *t,
        }
    }
}

pub fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

pub const DEFAULT_PORT: u16 = 7878;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_key() {
        let m = Message::Key { t: 1.5, key: "w".into(), state: KeyState::Down };
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(s, r#"{"type":"key","t":1.5,"key":"w","state":"down"}"#);
        let back: Message = serde_json::from_str(&s).unwrap();
        assert!(matches!(back, Message::Key { .. }));
    }

    #[test]
    fn round_trip_mouse_move() {
        let m = Message::MouseMove { t: 2.0, dx: 10, dy: -4 };
        let s = serde_json::to_string(&m).unwrap();
        let back: Message = serde_json::from_str(&s).unwrap();
        assert!(matches!(back, Message::MouseMove { dx: 10, dy: -4, .. }));
    }

    #[test]
    fn round_trip_mouse() {
        let m = Message::Mouse { t: 3.0, button: MouseButton::Left, state: ClickState::Click };
        let s = serde_json::to_string(&m).unwrap();
        let back: Message = serde_json::from_str(&s).unwrap();
        assert!(matches!(back, Message::Mouse { button: MouseButton::Left, state: ClickState::Click, .. }));
    }

    #[test]
    fn round_trip_heartbeat() {
        let m = Message::Heartbeat { t: 4.0 };
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(s, r#"{"type":"heartbeat","t":4.0}"#);
    }
}
