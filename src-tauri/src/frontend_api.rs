use crate::automator::ClickTarget;
use crate::shortcuts::ShortcutManager;
use crate::utils::{InputEvent, KeyCode, Modifier};
use crate::{resolve_state, set_active, Errors};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
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
    pub click_limit: u64,
    pub num_clicks: u64,
    pub is_limited: bool,
    pub target: ClickTarget,
}

#[tauri::command]
pub fn toggle_clicker_cmd(app: AppHandle) -> bool {
    let state = resolve_state(&app);
    let will_run = !state.is_running.load(Ordering::SeqCst);
    set_active(will_run, &app);
    will_run
}

#[tauri::command]
pub fn set_target_cmd(app: AppHandle, target: ClickTargetPayload) {
    match ClickTarget::from_payload(&target) {
        Some(t) => {
            let state = resolve_state(&app);
            *state.target.lock().unwrap() = t;
            println!("Set target: {:?}", state.target.lock().unwrap());
        }
        None => {
            let _ = app.emit("error", Errors::InvalidTarget);
        }
    }
}

#[tauri::command]
pub fn get_app_state_cmd(app: AppHandle) -> AppStateDto {
    resolve_state(&app).to_dto()
}

#[tauri::command]
pub fn set_cps_cmd(app: AppHandle, cps: f64) {
    resolve_state(&app)
        .cps
        .store(cps.to_bits(), Ordering::SeqCst);
}

#[tauri::command]
pub fn set_click_limit_cmd(app: AppHandle, limit: u64) {
    resolve_state(&app)
        .click_limit
        .store(limit, Ordering::SeqCst);
}

#[tauri::command]
pub fn set_num_clicks_cmd(app: AppHandle, num_clicks: u64) {
    resolve_state(&app)
        .num_clicks
        .store(num_clicks, Ordering::SeqCst);
}

#[tauri::command]
pub fn set_is_limited_cmd(app: AppHandle, is_limited: bool) {
    resolve_state(&app)
        .is_limited
        .store(is_limited, Ordering::SeqCst);
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
    ShortcutManager::update(&app, id, InputEvent::new(key_code, modifiers))
}

#[tauri::command]
pub fn is_wayland_cmd() -> bool {
    crate::utils::is_wayland()
}
