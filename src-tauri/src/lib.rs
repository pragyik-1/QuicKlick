use crate::automator::ClickTarget;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use tauri::{AppHandle, Emitter, Listener, Manager};

pub mod automator;
mod frontend_api;
pub mod shortcuts;
pub mod utils;

pub enum ClickerSig {
    Start,
    Stop,
}

#[derive(Clone, serde::Serialize)]
pub enum Errors {
    InvalidTarget,
}

#[derive(Clone, Serialize)]
pub struct AppState {
    pub is_running: Arc<AtomicBool>,
    pub delay: Arc<AtomicU64>,
    pub target: Arc<Mutex<ClickTarget>>,
    #[serde(skip)]
    pub tx: mpsc::Sender<ClickerSig>,
}

pub fn handle_action(id: &str, app: &AppHandle) {
    let state = app.state::<AppState>();
    match id {
        shortcuts::ACTION_TOGGLE => {
            let current = state.is_running.load(Ordering::SeqCst);
            set_active(!current, app);
        }
        shortcuts::ACTION_START => set_active(true, app),
        shortcuts::ACTION_STOP => set_active(false, app),
        _ => println!("Unknown action triggered: {}", id),
    }
}

pub fn set_active(is_active: bool, app: &AppHandle) {
    let state = app.state::<AppState>();
    let tx = app.state::<mpsc::Sender<ClickerSig>>();

    let _ = tx.send(if is_active {
        ClickerSig::Start
    } else {
        ClickerSig::Stop
    });

    state.is_running.store(is_active, Ordering::SeqCst);
    let _ = app.emit("state_change", state.inner().clone());
}

pub fn toggle_clicker(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let current = state.is_running.load(Ordering::SeqCst);
    set_active(!current, app);
    !current
}

pub fn set_target(target: ClickTarget, app: &AppHandle) {
    let state = app.state::<AppState>();
    *state.target.lock().unwrap() = target;
    println!("Set target: {:?}", state.target.lock().unwrap());
}

pub fn get_app_state(app: &AppHandle) -> AppState {
    let state = app.state::<AppState>();
    state.inner().clone()
}

pub fn set_delay(delay: u64, app: &AppHandle) {
    let state = app.state::<AppState>();
    state.delay.store(delay, Ordering::SeqCst);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(app_state: AppState, tx: mpsc::Sender<ClickerSig>) {
    let mut builder = tauri::Builder::default()
        .setup(move |app| {
            app.manage(app_state);
            app.manage(tx);

            let initial_map = shortcuts::ShortcutManager::default();
            shortcuts::ShortcutManager::init(app.handle(), initial_map);

            let app_clone = app.handle().clone();
            app.listen("shortcut_triggered", move |event| {
                if let Ok(id) = serde_json::from_str::<String>(event.payload()) {
                    handle_action(&id, &app_clone);
                } else {
                    let clean_id = event.payload().trim_matches('"');
                    if !clean_id.is_empty() {
                        handle_action(clean_id, &app_clone);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            frontend_api::update_shortcut_cmd,
            frontend_api::toggle_clicker_cmd,
            frontend_api::set_target_cmd,
            frontend_api::get_app_state_cmd,
            frontend_api::set_delay_cmd,
            frontend_api::is_wayland_cmd,
        ]);

    if !shortcuts::is_wayland() {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
