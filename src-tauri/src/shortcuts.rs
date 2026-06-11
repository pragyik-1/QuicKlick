use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::utils::{InputEvent, KeyCode, Modifier};

pub const ACTION_TOGGLE: &str = "toggle-clicker";
pub const ACTION_START: &str = "start-clicker";
pub const ACTION_STOP: &str = "stop-clicker";

pub fn is_wayland() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_SESSION_TYPE")
            .map(|val| val.to_lowercase() == "wayland")
            .unwrap_or_else(|_| std::env::var("WAYLAND_DISPLAY").is_ok())
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

pub fn get_action_description(id: &str) -> &'static str {
    match id {
        ACTION_TOGGLE => "Toggle Clicker",
        ACTION_START => "Start Clicker",
        ACTION_STOP => "Stop Clicker",
        _ => "AutoClicker Action",
    }
}

#[derive(Default)]
pub struct ShortcutManager {
    pub bindings: Mutex<HashMap<String, InputEvent>>,
    #[cfg(target_os = "linux")]
    pub wayland_task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl ShortcutManager {
    pub fn init(app: &AppHandle, initial_bindings: HashMap<String, InputEvent>) {
        let manager = Self {
            bindings: Mutex::new(initial_bindings.clone()),
            #[cfg(target_os = "linux")]
            wayland_task: Mutex::new(None),
        };
        app.manage(manager);

        #[cfg(target_os = "linux")]
        if is_wayland() {
            wayland::setup(app, initial_bindings);
            return;
        }

        standard::setup(app, initial_bindings);
    }

    pub fn default() -> HashMap<String, InputEvent> {
        HashMap::from([
            (
                ACTION_TOGGLE.to_string(),
                InputEvent::new(KeyCode::F(6), Vec::new()),
            ),
            (
                ACTION_START.to_string(),
                InputEvent::new(KeyCode::F(6), vec![Modifier::Shift]),
            ),
            (
                ACTION_STOP.to_string(),
                InputEvent::new(KeyCode::F(6), vec![Modifier::Control]),
            ),
        ])
    }

    pub fn update(app: &AppHandle, id: String, new_event: InputEvent) -> Result<(), String> {
        let state = app.state::<ShortcutManager>();
        let old_key = {
            let mut bindings = state.bindings.lock().unwrap();
            bindings.insert(id.clone(), new_event.clone())
        };

        #[cfg(target_os = "linux")]
        if is_wayland() {
            wayland::update(app)?;
            return Ok(());
        }

        standard::update(app, id, new_event, old_key)
    }
}

#[cfg(target_os = "linux")]
mod wayland {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use futures_util::StreamExt;
    use std::collections::HashMap;
    use tauri::{AppHandle, Emitter, Manager};

    use crate::shortcuts::{get_action_description, ShortcutManager};
    use crate::utils::{InputEvent, Modifier};

    pub fn format_portal_key(event: &InputEvent) -> String {
        let mut parts = Vec::new();
        if event.modifiers.contains(&Modifier::Meta) {
            parts.push("Meta");
        }
        if event.modifiers.contains(&Modifier::Control) {
            parts.push("Ctrl");
        }
        if event.modifiers.contains(&Modifier::Alt) {
            parts.push("Alt");
        }
        if event.modifiers.contains(&Modifier::Shift) {
            parts.push("Shift");
        }

        let key_name = event.key.to_string();
        parts.push(&key_name);
        parts.join("+")
    }

    pub fn setup(app: &AppHandle, bindings: HashMap<String, InputEvent>) {
        let state = app.state::<ShortcutManager>();
        let task = spawn_listener(app.clone(), bindings);
        *state.wayland_task.lock().unwrap() = Some(task);
    }

    pub fn update(app: &AppHandle) -> Result<(), String> {
        let state = app.state::<ShortcutManager>();
        let bindings = state.bindings.lock().unwrap().clone();

        if let Some(task) = state.wayland_task.lock().unwrap().take() {
            task.abort();
        }

        setup(app, bindings);
        Ok(())
    }

    fn spawn_listener(
        app: AppHandle,
        bindings: HashMap<String, InputEvent>,
    ) -> tauri::async_runtime::JoinHandle<()> {
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            let proxy = match GlobalShortcuts::new().await {
                Ok(p) => p,
                Err(e) => return eprintln!("Wayland shortcut proxy error: {}", e),
            };

            let session = match proxy.create_session().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Wayland shortcut session error: {}", e);
                    return;
                }
            };

            let portal_shorts: Vec<NewShortcut> = bindings
                .iter()
                .map(|(id, event): (&String, &InputEvent)| {
                    let formatted = format_portal_key(event);
                    NewShortcut::new(id.as_str(), get_action_description(id))
                        .preferred_trigger(&*formatted)
                })
                .collect();

            if let Err(e) = proxy
                .bind_shortcuts(
                    &session,
                    &portal_shorts,
                    &ashpd::WindowIdentifier::default(),
                )
                .await
            {
                return eprintln!("Failed to bind Wayland shortcuts: {}", e);
            }

            if let Ok(mut stream) = proxy.receive_activated().await {
                while let Some(sig) = stream.next().await {
                    let sig_id = sig.shortcut_id();
                    let state = app.state::<ShortcutManager>();
                    let has_key = state.bindings.lock().unwrap().contains_key(sig_id);
                    if has_key {
                        let _ = app.emit("shortcut_triggered", sig_id.to_string());
                    }
                }
            }
        })
    }
}

mod standard {
    use std::collections::HashMap;
    use std::str::FromStr;
    use tauri::{AppHandle, Emitter};
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

    use crate::utils::InputEvent;

    pub fn setup(app: &AppHandle, bindings: HashMap<String, InputEvent>) {
        for (id, key) in bindings {
            if let Err(e) = register(app, id, key) {
                eprintln!("{}", e);
            }
        }
    }

    pub fn update(
        app: &AppHandle,
        id: String,
        new_key: InputEvent,
        old_key: Option<InputEvent>,
    ) -> Result<(), String> {
        let manager = app.global_shortcut();

        if let Some(old_k) = old_key {
            if let Ok(s) = Shortcut::from_str(&old_k.to_string()) {
                let _ = manager.unregister(s);
            }
        }

        register(app, id, new_key)
    }

    fn register(app: &AppHandle, id: String, event: InputEvent) -> Result<(), String> {
        let manager = app.global_shortcut();
        let shortcut = Shortcut::from_str(&event.to_string())
            .map_err(|e| format!("Invalid key format for {}: {}", id, e))?;

        let id_clone = id.clone();
        manager
            .on_shortcut(shortcut, move |app_handle: &AppHandle, _, event| {
                if event.state == ShortcutState::Pressed {
                    let _ = app_handle.emit("shortcut_triggered", id_clone.clone());
                }
            })
            .map_err(|e| format!("Failed to register shortcut {}: {}", id, e))?;

        Ok(())
    }
}
