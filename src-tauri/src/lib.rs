use crate::automator::ClickTarget;
use crate::frontend_api::AppStateDto;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Listener, Manager};

pub mod automator;
mod click_loop;
mod frontend_api;
pub mod scheduler;
pub mod settings;
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

pub struct AppState {
    pub is_running: AtomicBool,
    pub cps: AtomicU64,
    pub click_limit: AtomicU64,
    pub num_clicks: AtomicU64,
    pub is_limited: AtomicBool,
    pub target: Mutex<ClickTarget>,
    pub mode: AtomicU8,
    pub sequence: Mutex<Vec<crate::automator::SeqTarget>>,
    pub repeat_sequence: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        use automator::{ClickType, Device, MouseButton};
        Self {
            is_running: AtomicBool::new(false),
            cps: AtomicU64::new(10.0f64.to_bits()),
            click_limit: AtomicU64::new(100),
            num_clicks: AtomicU64::new(0),
            is_limited: AtomicBool::new(false),
            target: Mutex::new(ClickTarget {
                key_code: Some(utils::KeyCode::Space),
                device: Device::Mouse,
                button: Some(MouseButton::Left),
                mouse_position: None,
                click_type: ClickType::Single,
                randomize_amount: None,
            }),
            mode: AtomicU8::new(0),
            sequence: Mutex::new(Vec::new()),
            repeat_sequence: AtomicBool::new(true),
        }
    }
}

impl AppState {
    pub fn to_dto(&self) -> AppStateDto {
        AppStateDto {
            is_running: self.is_running.load(Ordering::SeqCst),
            cps: f64::from_bits(self.cps.load(Ordering::SeqCst)),
            click_limit: self.click_limit.load(Ordering::SeqCst),
            num_clicks: self.num_clicks.load(Ordering::SeqCst),
            is_limited: self.is_limited.load(Ordering::SeqCst),
            target: self.target.lock().unwrap().clone(),
            mode: self.mode.load(Ordering::SeqCst),
            sequence: self.sequence.lock().unwrap().clone(),
            repeat_sequence: self.repeat_sequence.load(Ordering::SeqCst),
        }
    }

    pub fn emit(&self, app: &AppHandle) {
        let _ = app.emit("state_change", self.to_dto());
    }

    pub fn stop_and_emit(&self, app: &AppHandle) {
        self.is_running.store(false, Ordering::SeqCst);
        self.emit(app);
    }
}

pub fn resolve_state(app: &AppHandle) -> Arc<AppState> {
    app.state::<Arc<AppState>>().inner().clone()
}

pub fn handle_action(id: &str, app: &AppHandle) {
    match id {
        shortcuts::ACTION_TOGGLE => toggle_clicker(app),
        shortcuts::ACTION_START => set_active(true, app),
        shortcuts::ACTION_STOP => set_active(false, app),
        _ => println!("Unknown action triggered: {}", id),
    }
}

pub fn set_active(active: bool, app: &AppHandle) {
    let state = resolve_state(app);
    let tx = app.state::<mpsc::Sender<ClickerSig>>();

    let _ = tx.send(if active {
        ClickerSig::Start
    } else {
        ClickerSig::Stop
    });

    state.is_running.store(active, Ordering::SeqCst);
    if active {
        state.num_clicks.store(0, Ordering::SeqCst);
    }
    state.emit(app);
}

pub fn toggle_clicker(app: &AppHandle) {
    let running = resolve_state(app).is_running.load(Ordering::SeqCst);
    set_active(!running, app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(app_state: Arc<AppState>, tx: mpsc::Sender<ClickerSig>, rx: mpsc::Receiver<ClickerSig>) {
    let rx = Mutex::new(Some(rx));

    let mut builder = tauri::Builder::default()
        .setup(move |app| {
            app.manage(app_state.clone());
            app.manage(tx);

            let settings_mgr = settings::SettingsManager::new(app.handle());
            app.manage(settings_mgr.clone());

            let settings_data = settings_mgr.get();
            if settings_data.persist_app_state {
                if let Some(saved) = settings_data.saved_state {
                    app_state.cps.store(saved.cps.to_bits(), Ordering::SeqCst);
                    app_state
                        .click_limit
                        .store(saved.click_limit, Ordering::SeqCst);
                    app_state
                        .is_limited
                        .store(saved.is_limited, Ordering::SeqCst);
                    *app_state.target.lock().unwrap() = saved.target;
                    app_state.mode.store(saved.mode, Ordering::SeqCst);
                    *app_state.sequence.lock().unwrap() = saved.sequence;
                    app_state.repeat_sequence.store(saved.repeat_sequence, Ordering::SeqCst);
                }
            }

            shortcuts::ShortcutManager::init(
                app.handle(),
                settings_data.shortcuts,
                settings_data.use_evdev_shortcuts,
            );

            let handle = app.handle().clone();
            app.listen("shortcut_triggered", move |event| {
                let id = serde_json::from_str::<String>(event.payload())
                    .unwrap_or_else(|_| event.payload().trim_matches('"').to_string());
                if !id.is_empty() {
                    handle_action(&id, &handle);
                }
            });

            let click_app = app.handle().clone();
            let click_state = app_state.clone();
            let rx = rx.lock().unwrap().take().expect("rx already consumed");
            std::thread::spawn(move || click_loop::run(click_app, click_state, rx));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            frontend_api::toggle_clicker_cmd,
            frontend_api::set_target_cmd,
            frontend_api::get_app_state_cmd,
            frontend_api::set_cps_cmd,
            frontend_api::set_click_limit_cmd,
            frontend_api::set_num_clicks_cmd,
            frontend_api::set_is_limited_cmd,
            frontend_api::set_mode_cmd,
            frontend_api::set_sequence_cmd,
            frontend_api::set_repeat_sequence_cmd,
            frontend_api::update_shortcut_cmd,
            frontend_api::is_wayland_cmd,
            frontend_api::is_linux_cmd,
            settings::get_settings_cmd,
            settings::set_persist_app_state_cmd,
            settings::set_use_evdev_shortcuts_cmd,
            settings::save_preset_cmd,
            settings::delete_preset_cmd,
            settings::save_app_state_cmd,
        ]);

    if !utils::is_wayland() {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
