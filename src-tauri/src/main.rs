// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use quicklick_lib::AppState;
use std::sync::{mpsc, Arc};

fn main() {
    #[cfg(target_os = "linux")]
    // Fix weird rendering bugs in wayland.
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    let (tx, rx) = mpsc::channel();
    let app_state = Arc::new(AppState::default());

    quicklick_lib::run(app_state, tx, rx);
}
