use std::time::Duration;

use crate::frontend_api::ClickTargetPayload;
use crate::utils::{KeyCode, MacroTimer};
use enigo::{Enigo, Keyboard, Mouse, Settings};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

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
    Hold,
}

impl ClickType {
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "Single" => Some(Self::Single),
            "Double" => Some(Self::Double),
            "Randomized" => Some(Self::Randomized),
            "Hold" => Some(Self::Hold),
            _ => None,
        }
    }
}

/// The input a hold currently has down, so the release always matches the press
/// that started it and can be issued at most once.
#[derive(Debug, Clone, Copy)]
enum Held {
    Button(enigo::Button),
    Key(enigo::Key),
}

pub struct Automator<'a> {
    enigo: Enigo,
    timer: MacroTimer,
    last_pos: Option<(i32, i32)>,
    held: Option<Held>,
    app: &'a tauri::AppHandle,
}

impl<'a> Automator<'a> {
    const DOUBLE_CLICK_GAP: Duration = Duration::from_millis(15);

    pub fn new(app: &'a tauri::AppHandle) -> Result<Self, String> {
        let enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        Ok(Self {
            enigo,
            timer: MacroTimer::start(),
            last_pos: None,
            held: None,
            app,
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
        self.move_to(pos);
        let _ = self
            .enigo
            .button(button.to_enigo(), enigo::Direction::Click);
    }

    fn key_click(&mut self, key: &KeyCode) {
        if let Some(key) = self.enigo_key(key) {
            let _ = self.enigo.key(key, enigo::Direction::Click);
        }
    }

    /// Presses the target's input and leaves it down, so the click loop can hold
    /// it for a duration instead of releasing it straight away. The press is
    /// paired with [`Automator::end_hold`], which the loop calls on every exit
    /// path, including one interrupted by a stop.
    pub fn begin_hold(&mut self, target: &ClickTarget) {
        match target.device {
            Device::Mouse => {
                if let Some(button) = &target.button {
                    self.move_to(target.mouse_position);
                    let button = button.to_enigo();
                    match self.enigo.button(button, enigo::Direction::Press) {
                        Ok(()) => self.held = Some(Held::Button(button)),
                        // Reported rather than dropped: a press that never lands
                        // leaves this hold cycle doing nothing at all.
                        Err(e) => self.report(format!("Failed to hold mouse button: {e}")),
                    }
                }
            }
            Device::Keyboard => {
                if let Some(key) = &target.key_code {
                    if let Some(key) = self.enigo_key(key) {
                        match self.enigo.key(key, enigo::Direction::Press) {
                            Ok(()) => self.held = Some(Held::Key(key)),
                            Err(e) => self.report(format!("Failed to hold key: {e}")),
                        }
                    }
                }
            }
        }
    }

    /// Releases whatever [`Automator::begin_hold`] pressed. Releasing nothing, or
    /// releasing the same hold twice, does nothing
    pub fn end_hold(&mut self) {
        match self.held.take() {
            Some(Held::Button(button)) => {
                if let Err(e) = self.enigo.button(button, enigo::Direction::Release) {
                    self.report(format!("Failed to release mouse button: {e}"));
                }
            }
            Some(Held::Key(key)) => {
                if let Err(e) = self.enigo.key(key, enigo::Direction::Release) {
                    self.report(format!("Failed to release key: {e}"));
                }
            }
            None => {}
        }
    }

    /// Moves the pointer to the position the target pins, skipping the move when
    /// the pointer is already there.
    fn move_to(&mut self, pos: Option<(i32, i32)>) {
        if let Some((x, y)) = pos {
            if self.last_pos != Some((x, y)) {
                let _ = self.enigo.move_mouse(x, y, enigo::Coordinate::Abs);
                self.last_pos = Some((x, y));
            }
        }
    }

    fn enigo_key(&self, key: &KeyCode) -> Option<enigo::Key> {
        match key.to_enigo_key() {
            Some(k) => Some(k),
            None => {
                self.report(format!("Failed to convert key code {:?} to Enigo key", key));
                None
            }
        }
    }

    /// Hands a failure the user can act on to the frontend. A failed emit leaves
    /// only the process stderr to say so.
    fn report(&self, message: String) {
        if let Err(e) = self.app.emit("error", message) {
            eprintln!("Failed to deliver error to the frontend: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ClickType;

    /// Hold arrives from the frontend as a plain string, the same as every other
    /// click type, and an unknown name must not resolve to a click type.
    #[test]
    fn hold_parses_from_its_wire_name() {
        assert_eq!(ClickType::parse_str("Hold"), Some(ClickType::Hold));
        assert_eq!(ClickType::parse_str("Single"), Some(ClickType::Single));
        assert_eq!(ClickType::parse_str("Double"), Some(ClickType::Double));
        assert_eq!(
            ClickType::parse_str("Randomized"),
            Some(ClickType::Randomized)
        );
        assert_eq!(ClickType::parse_str("hold"), None);
        assert_eq!(ClickType::parse_str(""), None);
    }
}
