use crate::automator::ClickTarget;
use crate::shortcuts::ShortcutManager;
use crate::utils::{InputEvent, KeyCode, Modifier};
use crate::{get_app_state, set_cps, set_target, toggle_clicker, Errors};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ClickTargetPayload {
    pub device: String,
    pub key_code: Option<String>,
    pub button: Option<String>,
    pub mouse_position: Option<(i32, i32)>,
    pub click_type: String,
    pub randomize_amount: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppStateDto {
    pub is_running: bool,
    pub cps: f64,
    pub target: ClickTarget,
}

#[tauri::command]
pub fn toggle_clicker_cmd(app: AppHandle) -> bool {
    let running = toggle_clicker(&app);
    running
}

#[tauri::command]
pub fn set_target_cmd(app: AppHandle, target: ClickTargetPayload) {
    println!("Setting target: {:?}", target);
    if let Some(target) = ClickTarget::from_payload(&target) {
        set_target(target, &app);
    } else {
        let _ = app.emit("error", Errors::InvalidTarget);
    }
}

#[tauri::command]
pub fn update_shortcut_cmd(
    app: AppHandle,
    id: String,
    new_key: String,
    modifiers: Vec<String>,
) -> Result<(), String> {
    let modifiers = modifiers
        .iter()
        .filter_map(|s| Modifier::from_str(s))
        .collect();
    let key_code = KeyCode::from_str(Some(&new_key)).ok_or("Invalid key code")?;
    let event = InputEvent::new(key_code, modifiers);
    ShortcutManager::update(&app, id, event)
}

#[tauri::command]
pub fn get_app_state_cmd(app: AppHandle) -> AppStateDto {
    get_app_state(&app)
}

#[tauri::command]
pub fn set_cps_cmd(app: AppHandle, cps: f64) {
    set_cps(cps, &app);
}

#[tauri::command]
pub fn is_wayland_cmd() -> bool {
    crate::shortcuts::is_wayland()
}
