use std::time::Duration;

use crate::frontend_api::ClickTargetPayload;
use crate::utils::{KeyCode, MacroTimer};
use enigo::{Enigo, Keyboard, Mouse, Settings};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ClickTarget {
    pub key_code: Option<KeyCode>,
    pub device: Device,
    pub button: Option<MouseButton>,
    pub mouse_position: Option<(i32, i32)>,
    pub click_type: ClickType,
    pub randomize_amount: Option<u64>,
}

impl ClickTarget {
    pub fn from_payload(p: &ClickTargetPayload) -> Option<Self> {
        Some(Self {
            device: Device::parse_str(&p.device)?,
            button: MouseButton::parse_str(p.button.as_deref()),
            key_code: KeyCode::parse_str(p.key_code.as_deref()),
            mouse_position: p.mouse_position,
            click_type: ClickType::parse_str(&p.click_type)?,
            randomize_amount: p.randomize_amount,
        })
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SeqTarget {
    pub target: ClickTarget,
    pub wait_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Deserialize)]
pub enum Device {
    Mouse,
    Keyboard,
}

impl Device {
    fn parse_str(s: &str) -> Option<Self> {
        match s {
            "Mouse" => Some(Self::Mouse),
            "Keyboard" => Some(Self::Keyboard),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    fn to_enigo(&self) -> enigo::Button {
        match self {
            Self::Left => enigo::Button::Left,
            Self::Right => enigo::Button::Right,
            Self::Middle => enigo::Button::Middle,
        }
    }

    pub fn parse_str(s: Option<&str>) -> Option<Self> {
        match s? {
            "Left" => Some(Self::Left),
            "Right" => Some(Self::Right),
            "Middle" => Some(Self::Middle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClickType {
    Single,
    Double,
    Randomized,
}

impl ClickType {
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "Single" => Some(Self::Single),
            "Double" => Some(Self::Double),
            "Randomized" => Some(Self::Randomized),
            _ => None,
        }
    }
}
pub struct Automator {
    enigo: Enigo,
    timer: MacroTimer,
    last_pos: Option<(i32, i32)>,
}

impl Automator {
    const DOUBLE_CLICK_GAP: Duration = Duration::from_millis(15);

    pub fn new() -> Result<Self, String> {
        let enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        Ok(Self {
            enigo,
            timer: MacroTimer::start(),
            last_pos: None,
        })
    }

    pub fn handle_click(&mut self, target: &ClickTarget) {
        self.timer.reset();

        let is_double = target.click_type == ClickType::Double;

        match target.device {
            Device::Mouse => {
                if let Some(btn) = &target.button {
                    self.mouse_click(btn, target.mouse_position);
                    if is_double {
                        self.timer.sleep_remaining(Self::DOUBLE_CLICK_GAP);
                        self.mouse_click(btn, target.mouse_position);
                    }
                }
            }
            Device::Keyboard => {
                if let Some(key) = &target.key_code {
                    self.key_click(key);
                    if is_double {
                        self.timer.sleep_remaining(Self::DOUBLE_CLICK_GAP);
                        self.key_click(key);
                    }
                }
            }
        }
    }

    fn mouse_click(&mut self, button: &MouseButton, pos: Option<(i32, i32)>) {
        if let Some((x, y)) = pos {
            if self.last_pos != Some((x, y)) {
                let _ = self.enigo.move_mouse(x, y, enigo::Coordinate::Abs);
                self.last_pos = Some((x, y));
            }
        }
        let _ = self
            .enigo
            .button(button.to_enigo(), enigo::Direction::Click);
    }

    fn key_click(&mut self, key: &KeyCode) {
        let _ = self.enigo.key(key.to_enigo_key(), enigo::Direction::Click);
    }
}
