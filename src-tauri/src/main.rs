// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use quicklick_lib::{
    automator::{Automator, ClickTarget, ClickType, Device, MouseButton},
    scheduler, utils, AppState, ClickerSig,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

fn main() {
    #[cfg(target_os = "linux")]
    // Fix weird rendering bugs in wayland.
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    let (tx, rx) = mpsc::channel();

    let app_state: Arc<AppState> = Arc::new(AppState {
        is_running: AtomicBool::new(false),
        cps: AtomicU64::new(10.0f64.to_bits()),
        target: Mutex::new(ClickTarget {
            key_code: Some(utils::KeyCode::Space),
            device: Device::Mouse,
            button: Some(MouseButton::Left),
            mouse_position: None,
            click_type: ClickType::Single,
            randomize_amount: None,
        }),
    });

    let app_state_clone = app_state.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));

        let mut automator: Automator = Automator::new();
        let scheduler = scheduler::Scheduler::new(&rx);
        loop {
            if !app_state_clone.is_running.load(Ordering::SeqCst) {
                if let Ok(ClickerSig::Start) = rx.recv() {
                    app_state_clone.is_running.store(true, Ordering::SeqCst);
                } else {
                    break;
                }
            } else {
                let target = app_state_clone.target.lock().unwrap().clone();
                let cps = f64::from_bits(app_state_clone.cps.load(Ordering::SeqCst));

                let start_time = Instant::now();
                automator.handle_click(target.clone());
                let elapsed = start_time.elapsed();

                let should_continue = scheduler.wait_for_next_cycle(cps, &target, elapsed);

                if !should_continue {
                    app_state_clone.is_running.store(false, Ordering::SeqCst);
                }
            }
        }
    });
    quicklick_lib::run(app_state.clone(), tx);
}
