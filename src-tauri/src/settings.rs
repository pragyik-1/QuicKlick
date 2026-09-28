use crate::automator::ClickTarget;
use crate::shortcuts;
use crate::utils::InputEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub persist_app_state: bool,
    pub saved_state: Option<SavedState>,
    pub presets: HashMap<String, SavedState>,
    pub shortcuts: HashMap<String, InputEvent>,
    pub use_evdev_shortcuts: bool,
    /// Preset name bound to each preset shortcut slot, keyed by the slot's action
    /// id (`preset-1` .. `preset-5`). A slot missing from this map has no preset
    /// assigned. Absent on installs made before preset slots existed.
    #[serde(default)]
    pub preset_slots: HashMap<String, String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            persist_app_state: true,
            saved_state: None,
            presets: HashMap::new(),
            shortcuts: shortcuts::ShortcutManager::default(),
            use_evdev_shortcuts: false,
            preset_slots: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedState {
    pub cps: f64,
    pub click_limit: u64,
    pub is_limited: bool,
    pub target: ClickTarget,
    #[serde(default)]
    pub mode: u8,
    #[serde(default)]
    pub sequence: Vec<crate::automator::SeqTarget>,
    #[serde(default)]
    pub repeat_sequence: bool,
}

impl AppSettings {
    /// Points `slot` at the preset `name`, or clears the slot when `name` is empty.
    /// Rejects an id that is not a preset slot and a name that is not a saved preset,
    /// so a slot can never point at something that does not exist.
    pub fn set_preset_slot(&mut self, slot: &str, name: &str) -> Result<(), String> {
        if !shortcuts::is_preset_action(slot) {
            return Err(format!("unknown preset slot: {slot}"));
        }
        if name.is_empty() {
            self.preset_slots.remove(slot);
            return Ok(());
        }
        if !self.presets.contains_key(name) {
            return Err(format!("no preset named {name}"));
        }
        self.preset_slots.insert(slot.to_string(), name.to_string());
        Ok(())
    }

    /// Removes a saved preset and every slot pointing at it, so a slot cannot keep
    /// firing at a name that no longer resolves to a configuration.
    pub fn remove_preset(&mut self, name: &str) {
        self.presets.remove(name);
        self.preset_slots.retain(|_, preset| preset != name);
    }
}

pub struct SettingsManager {
    filepath: PathBuf,
    pub settings: Mutex<AppSettings>,
    app_handle: AppHandle,
}

impl SettingsManager {
    pub fn new(app: &AppHandle) -> Arc<Self> {
        let mut path = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::current_dir().unwrap());
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path.push("settings.json");

        let settings = if path.exists() {
            let data = fs::read_to_string(&path).unwrap_or_default();
            let mut s: AppSettings =
                serde_json::from_str(&data).unwrap_or_else(|_| AppSettings::default());
            let defaults = shortcuts::ShortcutManager::default();
            for (k, v) in defaults {
                s.shortcuts.entry(k).or_insert(v);
            }
            s
        } else {
            AppSettings::default()
        };

        Arc::new(Self {
            filepath: path,
            settings: Mutex::new(settings),
            app_handle: app.clone(),
        })
    }

    pub fn save(&self) {
        if let Ok(settings) = self.settings.lock() {
            let data = serde_json::to_string_pretty(&*settings).unwrap_or_default();
            let res = fs::write(&self.filepath, data);
            if let Err(e) = res {
                eprintln!("Failed to save settings: {}", e);
                let _ = self
                    .app_handle
                    .emit("error", format!("Failed to save settings: {}", e));
            }
        }
    }

    pub fn get(&self) -> AppSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut AppSettings),
    {
        if let Ok(mut settings) = self.settings.lock() {
            f(&mut settings);
        }
        self.save();
    }
}

#[tauri::command]
pub fn get_settings_cmd(app: AppHandle) -> AppSettings {
    let sm = app.state::<Arc<SettingsManager>>();
    sm.get()
}

#[tauri::command]
pub fn set_persist_app_state_cmd(app: AppHandle, persist: bool) {
    let sm = app.state::<Arc<SettingsManager>>();
    sm.update(|s| s.persist_app_state = persist);
}

#[tauri::command]
pub fn set_use_evdev_shortcuts_cmd(app: AppHandle, enabled: bool) {
    let sm = app.state::<Arc<SettingsManager>>();
    sm.update(|s| s.use_evdev_shortcuts = enabled);
}

#[tauri::command]
pub fn save_preset_cmd(app: AppHandle, name: String, state: SavedState) {
    let sm = app.state::<Arc<SettingsManager>>();
    sm.update(|s| {
        s.presets.insert(name, state);
    });
}

#[tauri::command]
pub fn delete_preset_cmd(app: AppHandle, name: String) {
    let sm = app.state::<Arc<SettingsManager>>();
    sm.update(|s| s.remove_preset(&name));
}

/// Assigns a saved preset to a preset shortcut slot. An empty `name` clears the
/// slot instead of binding it.
#[tauri::command]
pub fn set_preset_slot_cmd(app: AppHandle, slot: String, name: String) -> Result<(), String> {
    let sm = app.state::<Arc<SettingsManager>>();
    let mut settings = sm.settings.lock().map_err(|e| e.to_string())?;
    let result = settings.set_preset_slot(&slot, &name);
    drop(settings);
    if result.is_ok() {
        sm.save();
    }
    result
}

#[tauri::command]
pub fn save_app_state_cmd(app: AppHandle, state: SavedState) {
    let sm = app.state::<Arc<SettingsManager>>();
    sm.update(|s| {
        s.saved_state = Some(state);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automator::{ClickTarget, ClickType, Device, MouseButton};
    use crate::shortcuts;
    use crate::utils::{InputEvent, KeyCode, Modifier};

    fn sample_preset() -> SavedState {
        SavedState {
            cps: 25.0,
            click_limit: 500,
            is_limited: true,
            target: ClickTarget {
                key_code: None,
                device: Device::Mouse,
                button: Some(MouseButton::Left),
                mouse_position: None,
                click_type: ClickType::Single,
                randomize_amount: None,
            },
            mode: crate::utils::MODE_NORMAL,
            sequence: Vec::new(),
            repeat_sequence: true,
        }
    }

    fn settings_with_presets(names: &[&str]) -> AppSettings {
        let mut settings = AppSettings::default();
        for name in names {
            settings
                .presets
                .insert((*name).to_string(), sample_preset());
        }
        settings
    }

    /// A settings.json written before preset slots existed has no `preset_slots`
    /// key. It must still load, with every slot unbound.
    #[test]
    fn settings_without_preset_slots_loads_with_none_assigned() {
        let legacy = serde_json::json!({
            "persist_app_state": true,
            "saved_state": null,
            "presets": {},
            "shortcuts": {},
            "use_evdev_shortcuts": false,
        })
        .to_string();

        let parsed: AppSettings = serde_json::from_str(&legacy).expect("legacy settings must load");
        assert!(parsed.preset_slots.is_empty());
        for id in shortcuts::PRESET_ACTIONS {
            assert!(!parsed.shortcuts.contains_key(id));
        }
    }

    /// The saved shape must round-trip so a restart keeps the user's slot choices.
    #[test]
    fn preset_slots_round_trip_through_json() {
        let mut settings = settings_with_presets(&["fast", "slow"]);
        settings
            .set_preset_slot(shortcuts::ACTION_PRESET_2, "slow")
            .expect("slot accepts a saved preset");

        let encoded = serde_json::to_string(&settings).expect("settings must serialize");
        let decoded: AppSettings =
            serde_json::from_str(&encoded).expect("settings must deserialize");

        assert_eq!(
            decoded.preset_slots.get(shortcuts::ACTION_PRESET_2),
            Some(&"slow".to_string())
        );
        assert!(!decoded
            .preset_slots
            .contains_key(shortcuts::ACTION_PRESET_1));
    }

    #[test]
    fn slot_rejects_unknown_slot_id_and_missing_preset() {
        let mut settings = settings_with_presets(&["fast"]);

        assert!(settings.set_preset_slot("toggle-clicker", "fast").is_err());
        assert!(settings.set_preset_slot("preset-9", "fast").is_err());
        assert!(settings
            .set_preset_slot(shortcuts::ACTION_PRESET_1, "nope")
            .is_err());

        assert!(settings.preset_slots.is_empty());
    }

    /// Clearing a slot leaves no entry behind, which is how the UI shows "None".
    #[test]
    fn empty_name_clears_the_slot() {
        let mut settings = settings_with_presets(&["fast"]);
        settings
            .set_preset_slot(shortcuts::ACTION_PRESET_1, "fast")
            .expect("slot accepts a saved preset");

        settings
            .set_preset_slot(shortcuts::ACTION_PRESET_1, "")
            .expect("clearing an assigned slot succeeds");

        assert!(!settings
            .preset_slots
            .contains_key(shortcuts::ACTION_PRESET_1));
    }

    /// Deleting a preset clears it from every slot that referenced it, and leaves
    /// unrelated slots alone.
    #[test]
    fn deleting_a_preset_clears_the_slots_pointing_at_it() {
        let mut settings = settings_with_presets(&["fast", "slow"]);
        settings
            .set_preset_slot(shortcuts::ACTION_PRESET_1, "fast")
            .expect("slot accepts a saved preset");
        settings
            .set_preset_slot(shortcuts::ACTION_PRESET_3, "fast")
            .expect("slot accepts a saved preset");
        settings
            .set_preset_slot(shortcuts::ACTION_PRESET_5, "slow")
            .expect("slot accepts a saved preset");

        settings.remove_preset("fast");

        assert!(!settings.presets.contains_key("fast"));
        assert!(!settings
            .preset_slots
            .contains_key(shortcuts::ACTION_PRESET_1));
        assert!(!settings
            .preset_slots
            .contains_key(shortcuts::ACTION_PRESET_3));
        assert_eq!(
            settings.preset_slots.get(shortcuts::ACTION_PRESET_5),
            Some(&"slow".to_string())
        );
    }

    /// Adding the preset slots must not disturb the three shortcuts that ship with
    /// a default binding, since a fresh install registers exactly these keys.
    #[test]
    fn shortcut_defaults_still_register_on_first_run() {
        let settings = AppSettings::default();
        assert_eq!(
            settings.shortcuts.get(shortcuts::ACTION_TOGGLE),
            Some(&InputEvent::new(KeyCode::F(6), Vec::new()))
        );
        assert_eq!(
            settings.shortcuts.get(shortcuts::ACTION_START),
            Some(&InputEvent::new(KeyCode::F(6), vec![Modifier::Shift]))
        );
    }
}
