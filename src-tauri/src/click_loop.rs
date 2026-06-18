use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::AppHandle;

use crate::automator::Automator;
use crate::scheduler::Scheduler;
use crate::{AppState, ClickerSig};

/// Maximum frequency at which UI state updates are emitted during clicking.
const EMIT_INTERVAL: Duration = Duration::from_millis(50);

pub fn run(app: AppHandle, state: Arc<AppState>, rx: Receiver<ClickerSig>) {
    std::thread::sleep(Duration::from_secs(1));

    let mut automator = Automator::new();
    let scheduler = Scheduler::new(&rx);
    let mut last_emit = Instant::now();

    loop {
        if !state.is_running.load(Ordering::SeqCst) {
            match rx.recv() {
                Ok(ClickerSig::Start) => {
                    state.is_running.store(true, Ordering::SeqCst);
                    state.num_clicks.store(0, Ordering::SeqCst);
                    state.emit(&app);
                    last_emit = Instant::now();
                }
                _ => break,
            }
        } else {
            let target = state.target.lock().unwrap().clone();
            let cps = f64::from_bits(state.cps.load(Ordering::SeqCst));

            let start = Instant::now();
            automator.handle_click(&target);
            let elapsed = start.elapsed();

            if state.is_limited.load(Ordering::SeqCst) {
                let count = state.num_clicks.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= state.click_limit.load(Ordering::SeqCst) {
                    state.stop_and_emit(&app);
                    last_emit = Instant::now();
                    continue;
                }

                if last_emit.elapsed() >= EMIT_INTERVAL {
                    state.emit(&app);
                    last_emit = Instant::now();
                }
            }

            if !scheduler.wait_for_next_cycle(cps, &target, elapsed) {
                state.stop_and_emit(&app);
                last_emit = Instant::now();
            }
        }
    }
}
