use crate::automator::ClickTarget;
use crate::shortcuts::ShortcutManager;
use crate::utils::{InputEvent, KeyCode, Modifier};
use crate::{resolve_state, set_active, Errors};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ClickTargetPayload {
    pub device: String,
    pub key_code: Option<String>,
    pub button: Option<String>,
    pub mouse_position: Option<(i32, i32)>,
    pub click_type: String,
    pub randomize_amount: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SeqTargetPayload {
    pub target: ClickTargetPayload,
    pub wait_time: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppStateDto {
    pub is_running: bool,
    pub cps: f64,
    pub click_limit: u64,
    pub num_clicks: u64,
    pub is_limited: bool,
    pub target: ClickTarget,
    pub mode: u8,
    pub sequence: Vec<crate::automator::SeqTarget>,
    pub repeat_sequence: bool,
}

#[tauri::command]
pub fn toggle_clicker_cmd(app: AppHandle) -> bool {
    let state = resolve_state(&app);
    let will_run = !state.is_running.load(Ordering::SeqCst);
    set_active(will_run, &app);
    will_run
}

pub fn update_saved_state(app: &AppHandle) {
    let state = resolve_state(app);
    let sm = app.state::<Arc<crate::settings::SettingsManager>>();
    if sm.get().persist_app_state {
        sm.update(|s| {
            s.saved_state = Some(crate::settings::SavedState {
                cps: f64::from_bits(state.cps.load(Ordering::SeqCst)),
                click_limit: state.click_limit.load(Ordering::SeqCst),
                is_limited: state.is_limited.load(Ordering::SeqCst),
                target: state.target.lock().unwrap().clone(),
                mode: state.mode.load(Ordering::SeqCst),
                sequence: state.sequence.lock().unwrap().clone(),
                repeat_sequence: state.repeat_sequence.load(Ordering::SeqCst),
            });
        });
    }
}

#[tauri::command]
pub fn set_target_cmd(app: AppHandle, target: ClickTargetPayload) {
    match ClickTarget::from_payload(&target) {
        Some(t) => {
            let state = resolve_state(&app);
            *state.target.lock().unwrap() = t;
            println!("Set target: {:?}", state.target.lock().unwrap());
            update_saved_state(&app);
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
    update_saved_state(&app);
}

#[tauri::command]
pub fn set_click_limit_cmd(app: AppHandle, limit: u64) {
    resolve_state(&app)
        .click_limit
        .store(limit, Ordering::SeqCst);
    update_saved_state(&app);
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
    update_saved_state(&app);
}

#[tauri::command]
pub fn set_mode_cmd(app: AppHandle, mode: u8) {
    resolve_state(&app).mode.store(mode, Ordering::SeqCst);
    update_saved_state(&app);
}

#[tauri::command]
pub fn set_sequence_cmd(app: AppHandle, sequence: Vec<SeqTargetPayload>) {
    let mut parsed_sequence = Vec::new();
    for payload in sequence {
        if let Some(target) = ClickTarget::from_payload(&payload.target) {
            parsed_sequence.push(crate::automator::SeqTarget {
                target,
                wait_time: payload.wait_time,
            });
        }
    }
    
    let state = resolve_state(&app);
    *state.sequence.lock().unwrap() = parsed_sequence;
    update_saved_state(&app);
}

#[tauri::command]
pub fn set_repeat_sequence_cmd(app: AppHandle, repeat: bool) {
    resolve_state(&app).repeat_sequence.store(repeat, Ordering::SeqCst);
    update_saved_state(&app);
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

    ShortcutManager::update(&app, id.clone(), event.clone())?;

    let sm = app.state::<Arc<crate::settings::SettingsManager>>();
    sm.update(|s| {
        s.shortcuts.insert(id, event);
    });

    Ok(())
}

#[tauri::command]
pub fn is_wayland_cmd() -> bool {
    crate::utils::is_wayland()
}

#[tauri::command]
pub fn is_linux_cmd() -> bool {
    crate::utils::is_linux()
}