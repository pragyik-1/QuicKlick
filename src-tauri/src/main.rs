// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use quicklick_lib::{
    automator::{Automator, ClickTarget, ClickType, Device, MouseButton},
    utils, AppState, ClickerSig,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

fn main() {
    #[cfg(target_os = "linux")]
    // Fix weird rendering bugs in wayland.
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    let (tx, rx) = mpsc::channel();

    let app_state: AppState = AppState {
        is_running: Arc::new(AtomicBool::new(false)),
        delay: Arc::new(AtomicU64::new(500)),
        target: Arc::new(Mutex::new(ClickTarget {
            key_code: Some(utils::KeyCode::Space),
            device: Device::Mouse,
            button: Some(MouseButton::Left),
            mouse_position: None,
            click_type: ClickType::Single,
        })),
        tx: tx.clone(),
    };

    let app_state_clone = app_state.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));

        let mut automator: Automator = Automator::new();
        loop {
            if !app_state_clone.is_running.load(Ordering::SeqCst) {
                if let Ok(ClickerSig::Start) = rx.recv() {
                    app_state_clone.is_running.store(true, Ordering::SeqCst);
                } else {
                    break;
                }
            } else {
                let target: Arc<Mutex<ClickTarget>> = app_state_clone.target.clone();
                let delay = Duration::from_millis(app_state_clone.delay.load(Ordering::SeqCst));
                automator.handle_click(target.lock().unwrap().clone());

                match rx.recv_timeout(delay) {
                    Ok(ClickerSig::Stop) => {
                        app_state_clone.is_running.store(false, Ordering::SeqCst)
                    }
                    Ok(ClickerSig::Start) => (),
                    Err(mpsc::RecvTimeoutError::Timeout) => (),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        }
    });
    quicklick_lib::run(app_state, tx);
}
