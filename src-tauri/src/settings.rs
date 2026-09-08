use crate::automator::ClickTarget;
use crate::shortcuts;
use crate::utils::InputEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub persist_app_state: bool,
    pub saved_state: Option<SavedState>,
    pub presets: HashMap<String, SavedState>,
    pub shortcuts: HashMap<String, InputEvent>,
    #[serde(default = "default_use_evdev_shortcuts")]
    pub use_evdev_shortcuts: bool,
}

fn default_use_evdev_shortcuts() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            persist_app_state: true,
            saved_state: None,
            presets: HashMap::new(),
            shortcuts: shortcuts::ShortcutManager::default(),
            use_evdev_shortcuts: true,
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

pub struct SettingsManager {
    filepath: PathBuf,
    pub settings: Mutex<AppSettings>,
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
        })
    }

    pub fn save(&self) {
        if let Ok(settings) = self.settings.lock() {
            let data = serde_json::to_string_pretty(&*settings).unwrap_or_default();
            let _ = fs::write(&self.filepath, data);
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
    sm.update(|s| {
        s.presets.remove(&name);
    });
}

#[tauri::command]
pub fn save_app_state_cmd(app: AppHandle, state: SavedState) {
    let sm = app.state::<Arc<SettingsManager>>();
    sm.update(|s| {
        s.saved_state = Some(state);
    });
}
