use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use crate::automator::Automator;
use crate::scheduler::Scheduler;
use crate::{AppState, ClickerSig};

/// Maximum frequency at which UI state updates are emitted during clicking.
const EMIT_INTERVAL: Duration = Duration::from_millis(50);

pub fn run(app: AppHandle, state: Arc<AppState>, rx: Receiver<ClickerSig>) {
    std::thread::sleep(Duration::from_secs(1));

    let mut automator = match Automator::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to initialize input automation: {e}");
            let _ = app.emit(
                "error",
                format!("Failed to initialize input automation: {e}"),
            );
            return;
        }
    };
    let scheduler = Scheduler::new(&rx);
    let mut last_emit = Instant::now();
    let mut sequence_index: usize = 0;

    loop {
        if !state.is_running.load(Ordering::SeqCst) {
            match rx.recv() {
                Ok(ClickerSig::Start) => {
                    state.is_running.store(true, Ordering::SeqCst);
                    state.num_clicks.store(0, Ordering::SeqCst);
                    state.emit(&app);
                    last_emit = Instant::now();
                    sequence_index = 0;
                }
                Ok(ClickerSig::Stop) => {}
                Err(_) => break,
            }
        } else {
            let mode = state.mode.load(Ordering::SeqCst);

            if mode == 0 {
                // Normal mode
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
            } else {
                // Sequence mode
                let sequence = state.sequence.lock().unwrap().clone();
                if sequence.is_empty() {
                    state.stop_and_emit(&app);
                    continue;
                }

                let seq_target = &sequence[sequence_index % sequence.len()];
                let target = &seq_target.target;
                let wait_time = seq_target.wait_time.unwrap_or(1000);

                let start = Instant::now();
                automator.handle_click(target);
                let elapsed = start.elapsed();

                sequence_index += 1;

                if state.is_limited.load(Ordering::SeqCst) {
                    if sequence_index.is_multiple_of(sequence.len()) {
                        let count = state.num_clicks.fetch_add(1, Ordering::SeqCst) + 1;
                        if count >= state.click_limit.load(Ordering::SeqCst) {
                            state.stop_and_emit(&app);
                            last_emit = Instant::now();
                            continue;
                        }
                    }

                    if last_emit.elapsed() >= EMIT_INTERVAL {
                        state.emit(&app);
                        last_emit = Instant::now();
                    }
                }

                if !state.repeat_sequence.load(Ordering::SeqCst)
                    && sequence_index.is_multiple_of(sequence.len())
                {
                    state.stop_and_emit(&app);
                    last_emit = Instant::now();
                    continue;
                }

                let wait_duration = Duration::from_millis(wait_time).saturating_sub(elapsed);
                if !scheduler.wait_or_stop(wait_duration) {
                    state.stop_and_emit(&app);
                    last_emit = Instant::now();
                }
            }
        }
    }
}
