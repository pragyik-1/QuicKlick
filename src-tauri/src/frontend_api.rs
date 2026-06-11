use crate::automator::ClickTarget;
use crate::shortcuts::ShortcutManager;
use crate::utils::{InputEvent, KeyCode, Modifier};
use crate::{get_app_state, set_delay, set_target, toggle_clicker, AppState, Errors};
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct ClickTargetPayload {
    pub device: String,
    pub key_code: Option<String>,
    pub button: Option<String>,
    pub mouse_position: Option<(i32, i32)>,
    pub click_type: String,
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
pub fn get_app_state_cmd(app: AppHandle) -> AppState {
    get_app_state(&app)
}

#[tauri::command]
pub fn set_delay_cmd(app: AppHandle, delay: u64) {
    set_delay(delay, &app);
}

#[tauri::command]
pub fn is_wayland_cmd() -> bool {
    crate::shortcuts::is_wayland()
}
