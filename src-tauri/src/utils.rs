use std::thread;
use std::time::{Duration, Instant};

use enigo::Key;
use serde::{Deserialize, Serialize};

pub fn is_wayland() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_SESSION_TYPE")
            .map(|val| val.to_lowercase() == "wayland")
            .unwrap_or_else(|_| std::env::var("WAYLAND_DISPLAY").is_ok())
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Space,
    Enter,
    Tab,
    Backspace,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
    Escape,
    Insert,
    Delete,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modifier {
    Shift,
    Control,
    Alt,
    Meta,
}

impl Modifier {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ctrl" | "control" => Some(Modifier::Control),
            "shift" => Some(Modifier::Shift),
            "alt" => Some(Modifier::Alt),
            "meta" | "cmd" | "super" => Some(Modifier::Meta),
            _ => None,
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            Modifier::Shift => "Shift".to_string(),
            Modifier::Control => "Ctrl".to_string(),
            Modifier::Alt => "Alt".to_string(),
            Modifier::Meta => "Meta".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InputEvent {
    pub key: KeyCode,
    pub modifiers: Vec<Modifier>,
}

impl InputEvent {
    pub fn new(key: KeyCode, modifiers: Vec<Modifier>) -> Self {
        Self { key, modifiers }
    }
    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();
        for modifier in &self.modifiers {
            parts.push(modifier.to_string());
        }
        parts.push(self.key.to_string());
        parts.join("+")
    }
}

impl KeyCode {
    pub fn to_string(&self) -> String {
        match self {
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Space => "space".to_string(),
            KeyCode::Enter => "enter".to_string(),
            KeyCode::Tab => "tab".to_string(),
            KeyCode::Backspace => "backspace".to_string(),
            KeyCode::Left => "left".to_string(),
            KeyCode::Right => "right".to_string(),
            KeyCode::Up => "up".to_string(),
            KeyCode::Down => "down".to_string(),
            KeyCode::Home => "home".to_string(),
            KeyCode::End => "end".to_string(),
            KeyCode::PageUp => "pageup".to_string(),
            KeyCode::PageDown => "pagedown".to_string(),
            KeyCode::F(n) => format!("f{}", n),
            KeyCode::Escape => "escape".to_string(),
            KeyCode::Insert => "insert".to_string(),
            KeyCode::Delete => "delete".to_string(),
            KeyCode::Unknown(s) => s.clone(),
        }
    }

    pub fn to_enigo_key(&self) -> enigo::Key {
        match self {
            KeyCode::Space => Key::Space,
            KeyCode::Enter => Key::Return,
            KeyCode::Tab => Key::Tab,
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Left => Key::LeftArrow,
            KeyCode::Right => Key::RightArrow,
            KeyCode::Up => Key::UpArrow,
            KeyCode::Down => Key::DownArrow,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            KeyCode::PageUp => Key::PageUp,
            KeyCode::PageDown => Key::PageDown,
            KeyCode::Escape => Key::Escape,
            KeyCode::Insert => Key::Insert,
            KeyCode::Delete => Key::Delete,
            KeyCode::F(n) => match n {
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
                _ => Key::Control, // TODO: Handle this case properly.
            },
            KeyCode::Char(c) => Key::Unicode(*c),
            KeyCode::Unknown(s) => Key::Unicode(s.chars().next().unwrap()),
        }
    }
    pub fn from_str(code_str: Option<&str>) -> Option<Self> {
        if code_str.is_none() {
            return None;
        }
        let code_str = code_str.unwrap();
        let normalized = code_str.to_lowercase();

        let key = match normalized.as_str() {
            "space" => KeyCode::Space,
            "enter" => KeyCode::Enter,
            "tab" => KeyCode::Tab,
            "backspace" => KeyCode::Backspace,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            "escape" | "esc" => KeyCode::Escape,
            "insert" => KeyCode::Insert,
            "delete" => KeyCode::Delete,
            s if s.starts_with('f') && s[1..].parse::<u8>().is_ok() => {
                KeyCode::F(s[1..].parse().unwrap())
            }
            s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
            _ => return None,
        };
        Some(key)
    }
}

impl<'de> Deserialize<'de> for KeyCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        KeyCode::from_str(Some(&s))
            .ok_or_else(|| serde::de::Error::custom(format!("invalid key code: {}", s)))
    }
}

impl serde::Serialize for KeyCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub struct MacroTimer {
    last_tick: Instant,
}

impl MacroTimer {
    pub fn start() -> Self {
        Self {
            last_tick: Instant::now(),
        }
    }
    pub fn reset(&mut self) {
        self.last_tick = Instant::now();
    }
    pub fn sleep_remaining(&mut self, target_duration: Duration) {
        let elapsed = self.last_tick.elapsed();

        if let Some(sleep_time) = target_duration.checked_sub(elapsed) {
            thread::sleep(sleep_time);
            self.last_tick += target_duration;
        } else {
            self.last_tick = Instant::now();
        }
    }
}
