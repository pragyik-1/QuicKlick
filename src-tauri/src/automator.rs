use std::time::Duration;

use crate::{
    frontend_api::ClickTargetPayload,
    utils::{KeyCode, MacroTimer},
    Errors,
};
use enigo::{Enigo, Keyboard, Mouse, Settings};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ClickTarget {
    pub key_code: Option<KeyCode>,
    pub device: Device,
    pub button: Option<MouseButton>,
    pub mouse_position: Option<(i32, i32)>,
    pub click_type: ClickType,
}

impl ClickTarget {
    pub fn from_payload(payload: &ClickTargetPayload) -> Option<Self> {
        let device: Device = Device::from_str(&payload.device)?;
        let button = MouseButton::from_str(payload.button.as_deref());
        let key_code = KeyCode::from_str(payload.key_code.as_deref());
        let mouse_position = payload.mouse_position;
        let click_type = ClickType::from_str(&payload.click_type)?;
        Some(Self {
            key_code,
            device,
            button,
            mouse_position,
            click_type,
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Deserialize)]
pub enum Device {
    Mouse,
    Keyboard,
}

impl Device {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "Mouse" => Some(Device::Mouse),
            "Keyboard" => Some(Device::Keyboard),
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
    pub fn to_enigo_button(&self) -> enigo::Button {
        match self {
            MouseButton::Left => enigo::Button::Left,
            MouseButton::Right => enigo::Button::Right,
            MouseButton::Middle => enigo::Button::Middle,
        }
    }
    pub fn from_str(s: Option<&str>) -> Option<Self> {
        if s.is_none() {
            return None;
        }
        let s = s.unwrap();
        match s {
            "Left" => Some(MouseButton::Left),
            "Right" => Some(MouseButton::Right),
            "Middle" => Some(MouseButton::Middle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClickType {
    Single,
    Double,
    Randomized,
}

impl ClickType {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "Single" => Some(ClickType::Single),
            "Double" => Some(ClickType::Double),
            "Randomized" => Some(ClickType::Randomized),
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
    const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(15);
    pub fn new() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default()).expect("Failed to initialize Engio"),
            timer: MacroTimer::start(),
            last_pos: None,
        }
    }
    pub fn handle_click(&mut self, target: ClickTarget) {
        self.timer.reset();
        match target.device {
            Device::Mouse => {
                if let Some(button) = target.button {
                    match target.click_type {
                        ClickType::Single => {
                            let _ = self.mouse_click(button, target.mouse_position);
                        }
                        ClickType::Double => {
                            let button_clone = button.clone();
                            let _ = self.mouse_click(button, target.mouse_position);
                            self.timer.sleep_remaining(Self::DOUBLE_CLICK_INTERVAL);
                            let _ = self.mouse_click(button_clone, target.mouse_position);
                        }
                        ClickType::Randomized => todo!("Randomized click not implemented yet"),
                    }
                }
            }
            Device::Keyboard => {
                if let Some(key) = target.key_code {
                    match target.click_type {
                        ClickType::Single => self.key_click(key),
                        ClickType::Double => {
                            let key_clone = key.clone();
                            let _ = self.key_click(key);
                            self.timer.sleep_remaining(Self::DOUBLE_CLICK_INTERVAL);
                            let _ = self.key_click(key_clone);
                        }
                        ClickType::Randomized => todo!("Randomized click not implemented yet"),
                    }
                }
            }
        }
    }
    fn mouse_click(&mut self, button: MouseButton, pos: Option<(i32, i32)>) -> Result<(), Errors> {
        if let Some((x, y)) = pos {
            if self.last_pos != Some((x, y)) {
                let _ = self.enigo.move_mouse(x, y, enigo::Coordinate::Abs);
                self.last_pos = Some((x, y));
            }
        }
        let _ = self
            .enigo
            .button(button.to_enigo_button(), enigo::Direction::Click);
        Ok(())
    }
    fn key_click(&mut self, key: KeyCode) {
        self.enigo
            .key(key.to_enigo_key(), enigo::Direction::Click)
            .unwrap();
    }
}
