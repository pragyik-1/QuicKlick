use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::utils::{is_wayland, InputEvent, KeyCode, Modifier};

pub const ACTION_TOGGLE: &str = "toggle-clicker";
pub const ACTION_START: &str = "start-clicker";
pub const ACTION_STOP: &str = "stop-clicker";
pub const ACTION_PRESET_1: &str = "preset-1";
pub const ACTION_PRESET_2: &str = "preset-2";
pub const ACTION_PRESET_3: &str = "preset-3";
pub const ACTION_PRESET_4: &str = "preset-4";
pub const ACTION_PRESET_5: &str = "preset-5";

/// The preset slot action ids in slot order, so a slot number indexes this directly.
pub const PRESET_ACTIONS: [&str; 5] = [
    ACTION_PRESET_1,
    ACTION_PRESET_2,
    ACTION_PRESET_3,
    ACTION_PRESET_4,
    ACTION_PRESET_5,
];

pub fn is_preset_action(id: &str) -> bool {
    PRESET_ACTIONS.contains(&id)
}

pub fn get_action_description(id: &str) -> &'static str {
    match id {
        ACTION_TOGGLE => "Toggle Clicker",
        ACTION_START => "Start Clicker",
        ACTION_STOP => "Stop Clicker",
        ACTION_PRESET_1 => "Preset 1",
        ACTION_PRESET_2 => "Preset 2",
        ACTION_PRESET_3 => "Preset 3",
        ACTION_PRESET_4 => "Preset 4",
        ACTION_PRESET_5 => "Preset 5",
        _ => "AutoClicker Action",
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LinuxBackend {
    Portal,
    Evdev,
}

#[derive(Default)]
pub struct ShortcutManager {
    pub bindings: Mutex<HashMap<String, InputEvent>>,
    #[cfg(target_os = "linux")]
    pub backend: Mutex<Option<LinuxBackend>>,
    #[cfg(target_os = "linux")]
    pub wayland_task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    #[cfg(target_os = "linux")]
    pub kbd_manager: Mutex<Option<evdev_shortcuts::KbdShortcutManager>>,
}

impl ShortcutManager {
    pub fn init(
        app: &AppHandle,
        initial_bindings: HashMap<String, InputEvent>,
        use_evdev_shortcuts: bool,
    ) {
        let manager = Self {
            bindings: Mutex::new(initial_bindings.clone()),
            #[cfg(target_os = "linux")]
            backend: Mutex::new(None),
            #[cfg(target_os = "linux")]
            wayland_task: Mutex::new(None),
            #[cfg(target_os = "linux")]
            kbd_manager: Mutex::new(None),
        };
        app.manage(manager);

        #[cfg(target_os = "linux")]
        {
            let state = app.state::<ShortcutManager>();
            if use_evdev_shortcuts {
                if evdev_permissions::ensure(app)
                    && evdev_shortcuts::try_setup(app, initial_bindings.clone()).is_ok()
                {
                    *state.backend.lock().unwrap() = Some(LinuxBackend::Evdev);
                    return;
                }
                eprintln!("evdev shortcuts unavailable, falling back to portal");
            }
            if is_wayland() {
                *state.backend.lock().unwrap() = Some(LinuxBackend::Portal);
                wayland::setup(app, initial_bindings);
                return;
            }
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
        let old_key = state.bindings.lock().unwrap().get(&id).cloned();

        #[cfg(target_os = "linux")]
        {
            match *state.backend.lock().unwrap() {
                Some(LinuxBackend::Portal) => {
                    state.bindings.lock().unwrap().insert(id, new_event);
                    return wayland::update(app);
                }
                Some(LinuxBackend::Evdev) => {
                    state.bindings.lock().unwrap().insert(id, new_event);
                    return evdev_shortcuts::update(app);
                }
                None => {}
            }
        }

        standard::update(app, id.clone(), new_event.clone(), old_key)?;
        state.bindings.lock().unwrap().insert(id, new_event);
        Ok(())
    }

    /// Drops a binding entirely. An action with no entry in `bindings` is unbound,
    /// which is how the preset slots start out and how a cleared shortcut is stored.
    pub fn unbind(app: &AppHandle, id: &str) -> Result<(), String> {
        let state = app.state::<ShortcutManager>();
        let old_key = state.bindings.lock().unwrap().remove(id);

        #[cfg(target_os = "linux")]
        {
            match *state.backend.lock().unwrap() {
                Some(LinuxBackend::Portal) => return wayland::update(app),
                Some(LinuxBackend::Evdev) => return evdev_shortcuts::update(app),
                None => {}
            }
        }

        standard::unbind(app, old_key)
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
            fn fallback_to_evdev(app: &AppHandle, bindings: &HashMap<String, InputEvent>) {
                eprintln!("Wayland portal unavailable; falling back to evdev shortcuts");
                if super::evdev_shortcuts::enable_as_fallback(app, bindings.clone()) {
                    let _ = app.emit(
                        "error",
                        "Wayland shortcuts unavailable; using evdev shortcuts.",
                    );
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            let proxy = match GlobalShortcuts::new().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Wayland shortcut proxy error: {}", e);
                    fallback_to_evdev(&app, &bindings);
                    return;
                }
            };

            let session = match proxy.create_session().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Wayland shortcut session error: {}", e);
                    fallback_to_evdev(&app, &bindings);
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
                eprintln!("Failed to bind Wayland shortcuts: {}", e);
                fallback_to_evdev(&app, &bindings);
                return;
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

#[cfg(target_os = "linux")]
pub mod evdev_shortcuts {
    use std::collections::HashMap;

    use kbd::hotkey::{Hotkey, Modifier as KbdModifier};
    use kbd::key::Key;
    use kbd_global::binding_guard::BindingGuard;
    use kbd_global::manager::HotkeyManager;
    use tauri::{AppHandle, Emitter, Manager};

    use crate::shortcuts::ShortcutManager;
    use crate::utils::{InputEvent, KeyCode, Modifier};

    pub struct KbdShortcutManager {
        pub manager: HotkeyManager,
        pub guards: Vec<BindingGuard>,
    }

    pub fn try_setup(app: &AppHandle, bindings: HashMap<String, InputEvent>) -> Result<(), String> {
        let manager = HotkeyManager::new().map_err(|e| format!("{e}"))?;

        let mut guards = Vec::new();
        for (id, event) in bindings {
            match register(&manager, app, &id, &event) {
                Ok(guard) => guards.push(guard),
                Err(e) => return Err(e),
            }
        }

        let state = app.state::<ShortcutManager>();
        *state.kbd_manager.lock().unwrap() = Some(KbdShortcutManager { manager, guards });
        Ok(())
    }

    /// Called when the Wayland portal is unavailable but evdev shortcuts should
    /// still be used as a fallback. Returns `true` if evdev is now active.
    pub fn enable_as_fallback(app: &AppHandle, bindings: HashMap<String, InputEvent>) -> bool {
        if !super::evdev_permissions::ensure(app) {
            return false;
        }
        if let Err(e) = try_setup(app, bindings) {
            eprintln!("evdev fallback failed: {e}");
            return false;
        }
        let state = app.state::<ShortcutManager>();
        *state.backend.lock().unwrap() = Some(super::LinuxBackend::Evdev);
        true
    }

    fn register(
        manager: &HotkeyManager,
        app: &AppHandle,
        id: &str,
        event: &InputEvent,
    ) -> Result<BindingGuard, String> {
        let hotkey = input_event_to_hotkey(event)?;
        let id_clone = id.to_string();
        let app_clone = app.clone();

        manager
            .register(hotkey, move || {
                let _ = app_clone.emit("shortcut_triggered", id_clone.clone());
            })
            .map_err(|e| format!("Failed to register shortcut {id}: {e}"))
    }

    pub fn update(app: &AppHandle) -> Result<(), String> {
        let state = app.state::<ShortcutManager>();
        let bindings = state.bindings.lock().unwrap().clone();

        let mut kbd_lock = state.kbd_manager.lock().unwrap();
        if let Some(kbd) = kbd_lock.as_mut() {
            kbd.guards.clear();
            let mut fresh = Vec::new();
            for (id, event) in bindings {
                match register(&kbd.manager, app, &id, &event) {
                    Ok(guard) => fresh.push(guard),
                    Err(e) => return Err(e),
                }
            }
            kbd.guards = fresh;
        }
        Ok(())
    }

    fn input_event_to_hotkey(event: &InputEvent) -> Result<Hotkey, String> {
        let key = to_kbd_key(&event.key);
        let mut hotkey = Hotkey::new(key);
        for modifier in &event.modifiers {
            hotkey = hotkey.modifier(to_kbd_modifier(modifier));
        }
        Ok(hotkey)
    }

    fn to_kbd_modifier(m: &Modifier) -> KbdModifier {
        match m {
            Modifier::Shift => KbdModifier::Shift,
            Modifier::Control => KbdModifier::Ctrl,
            Modifier::Alt => KbdModifier::Alt,
            Modifier::Meta => KbdModifier::Super,
        }
    }

    fn to_kbd_key(key: &KeyCode) -> Key {
        match key {
            KeyCode::Space => Key::SPACE,
            KeyCode::Enter => Key::ENTER,
            KeyCode::Tab => Key::TAB,
            KeyCode::Backspace => Key::BACKSPACE,
            KeyCode::Left => Key::ARROW_LEFT,
            KeyCode::Right => Key::ARROW_RIGHT,
            KeyCode::Up => Key::ARROW_UP,
            KeyCode::Down => Key::ARROW_DOWN,
            KeyCode::Home => Key::HOME,
            KeyCode::End => Key::END,
            KeyCode::PageUp => Key::PAGE_UP,
            KeyCode::PageDown => Key::PAGE_DOWN,
            KeyCode::Escape => Key::ESCAPE,
            KeyCode::Insert => Key::INSERT,
            KeyCode::Delete => Key::DELETE,
            KeyCode::F(1) => Key::F1,
            KeyCode::F(2) => Key::F2,
            KeyCode::F(3) => Key::F3,
            KeyCode::F(4) => Key::F4,
            KeyCode::F(5) => Key::F5,
            KeyCode::F(6) => Key::F6,
            KeyCode::F(7) => Key::F7,
            KeyCode::F(8) => Key::F8,
            KeyCode::F(9) => Key::F9,
            KeyCode::F(10) => Key::F10,
            KeyCode::F(11) => Key::F11,
            KeyCode::F(12) => Key::F12,
            KeyCode::F(_) => Key::UNIDENTIFIED,
            KeyCode::Char(c) => char_to_key(*c).unwrap_or(Key::UNIDENTIFIED),
            KeyCode::Unknown(_) => Key::UNIDENTIFIED,
        }
    }

    fn char_to_key(c: char) -> Option<Key> {
        match c {
            'a' | 'A' => Some(Key::A),
            'b' | 'B' => Some(Key::B),
            'c' | 'C' => Some(Key::C),
            'd' | 'D' => Some(Key::D),
            'e' | 'E' => Some(Key::E),
            'f' | 'F' => Some(Key::F),
            'g' | 'G' => Some(Key::G),
            'h' | 'H' => Some(Key::H),
            'i' | 'I' => Some(Key::I),
            'j' | 'J' => Some(Key::J),
            'k' | 'K' => Some(Key::K),
            'l' | 'L' => Some(Key::L),
            'm' | 'M' => Some(Key::M),
            'n' | 'N' => Some(Key::N),
            'o' | 'O' => Some(Key::O),
            'p' | 'P' => Some(Key::P),
            'q' | 'Q' => Some(Key::Q),
            'r' | 'R' => Some(Key::R),
            's' | 'S' => Some(Key::S),
            't' | 'T' => Some(Key::T),
            'u' | 'U' => Some(Key::U),
            'v' | 'V' => Some(Key::V),
            'w' | 'W' => Some(Key::W),
            'x' | 'X' => Some(Key::X),
            'y' | 'Y' => Some(Key::Y),
            'z' | 'Z' => Some(Key::Z),
            '0' => Some(Key::DIGIT0),
            '1' => Some(Key::DIGIT1),
            '2' => Some(Key::DIGIT2),
            '3' => Some(Key::DIGIT3),
            '4' => Some(Key::DIGIT4),
            '5' => Some(Key::DIGIT5),
            '6' => Some(Key::DIGIT6),
            '7' => Some(Key::DIGIT7),
            '8' => Some(Key::DIGIT8),
            '9' => Some(Key::DIGIT9),
            ' ' => Some(Key::SPACE),
            _ => None,
        }
    }
}

#[cfg(target_os = "linux")]
pub mod evdev_permissions {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::process::Command;

    use tauri::{AppHandle, Emitter};

    const UDEV_RULE: &str = "/etc/udev/rules.d/99-quicklick-input.rules";

    /// Whether the running user can already read input event devices.
    fn devices_readable() -> bool {
        fs::read_dir("/dev/input")
            .map(|entries| {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !name.starts_with("event") {
                        continue;
                    }
                    if let Ok(f) = fs::File::open(entry.path()) {
                        drop(f);
                        return true;
                    }
                }
                false
            })
            .unwrap_or(false)
    }

    /// A udev rule that grants (setfacl) the logged-in user read access to
    /// existing and newly created input devices, plus immediate application.
    fn setup_script(username: &str) -> String {
        format!(
            "#!/bin/bash\n\
             set -e\n\
             RULE='/etc/udev/rules.d/99-quicklick-input.rules'\n\
             cat > \"$RULE\" <<'EOF'\n\
             KERNEL==\"event*\", SUBSYSTEM==\"input\", RUN+=\"/usr/bin/setfacl -m u:{username}:r /dev/input/event*\"\n\
             EOF\n\
             chmod 644 \"$RULE\"\n\
             udevadm control --reload-rules\n\
             udevadm trigger --subsystem-match=input\n\
             setfacl -m u:{username}:r /dev/input/event* 2>/dev/null || true\n",
            username = username
        )
    }

    /// Ask the user (via pkexec) to install permissions so we can read
    /// `/dev/input/event*`. Returns true if access is available afterward.
    pub fn ensure(app: &AppHandle) -> bool {
        if devices_readable() {
            return true;
        }

        let username = match std::env::var("USER") {
            Ok(u) => u,
            Err(_) => return false,
        };

        let script_path = std::env::temp_dir().join("quicklick-setup-evdev.sh");
        if fs::write(&script_path, setup_script(&username)).is_err() {
            return false;
        }
        let _ = fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755));

        let result = Command::new("pkexec")
            .arg("/bin/bash")
            .arg(&script_path)
            .output();

        let _ = fs::remove_file(&script_path);

        match result {
            Ok(output) if output.status.success() => {
                let ok = devices_readable();
                if !ok {
                    let _ = app.emit("error", "evdev permission setup ran but /dev/input is still not readable. Try logging out and back in.");
                }
                ok
            }
            Ok(_) => {
                let _ = app.emit("error", "evdev permission setup was cancelled or failed.");
                false
            }
            Err(e) => {
                let _ = app.emit(
                    "error",
                    format!("pkexec not available for evdev permission setup: {e}"),
                );
                false
            }
        }
    }

    #[allow(dead_code)]
    fn _udev_rule_exists() -> bool {
        Path::new(UDEV_RULE).exists()
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

        if old_key.as_ref() == Some(&new_key) {
            return Ok(());
        }
        register(app, id, new_key)?;

        if let Some(old_k) = old_key {
            if let Ok(s) = Shortcut::from_str(&old_k.to_string()) {
                let _ = manager.unregister(s);
            }
        }

        Ok(())
    }

    /// Releases the accelerator an action was holding, if any. Without the key the
    /// action cannot be unregistered, so a never-bound or already-cleared action
    /// is a no-op success.
    pub fn unbind(app: &AppHandle, old_key: Option<InputEvent>) -> Result<(), String> {
        let Some(old_key) = old_key else {
            return Ok(());
        };
        let shortcut = Shortcut::from_str(&old_key.to_string())
            .map_err(|e| format!("Invalid key format to unregister: {}", e))?;
        app.global_shortcut()
            .unregister(shortcut)
            .map_err(|e| format!("Failed to unregister shortcut: {}", e))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Preset slot ids must round-trip through the id the frontend sends when it
    /// assigns a preset to a slot, so a slot can never be addressed by two names.
    #[test]
    fn preset_slot_ids_are_unique_and_described() {
        let ids = [
            ACTION_PRESET_1,
            ACTION_PRESET_2,
            ACTION_PRESET_3,
            ACTION_PRESET_4,
            ACTION_PRESET_5,
        ];

        let mut sorted = ids.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "preset slot ids must be unique");

        for id in ids {
            assert_eq!(
                get_action_description(id),
                format!("Preset {}", &id["preset-".len()..]),
                "every preset slot needs the label shown in the portal dialog"
            );
        }
    }

    /// New installs and older settings files both start with the preset slots
    /// unbound. `SettingsManager::new` only backfills what `default()` contains, so
    /// keeping the preset ids out of `default()` is what leaves a pre-existing
    /// `settings.json` without preset bindings.
    #[test]
    fn defaults_leave_preset_slots_unbound() {
        let defaults = ShortcutManager::default();
        for id in [
            ACTION_PRESET_1,
            ACTION_PRESET_2,
            ACTION_PRESET_3,
            ACTION_PRESET_4,
            ACTION_PRESET_5,
        ] {
            assert!(
                !defaults.contains_key(id),
                "{id} must have no default binding"
            );
        }
        for id in [ACTION_TOGGLE, ACTION_START, ACTION_STOP] {
            assert!(
                defaults.contains_key(id),
                "{id} must keep its default binding"
            );
        }
    }
}
