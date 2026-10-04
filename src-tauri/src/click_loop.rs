use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use crate::automator::{Automator, ClickType};
use crate::scheduler::{self, Scheduler};
use crate::{AppState, ClickerSig};

/// Maximum frequency at which UI state updates are emitted during clicking.
const EMIT_INTERVAL: Duration = Duration::from_millis(50);
pub(crate) const MODE_NORMAL: u8 = 0;
pub(crate) const MODE_SEQUENCE: u8 = 1;

/// Completes a hold whose press has already been dispatched: waits out `press`,
/// then releases the input and leaves the release gap the next hold needs to
/// register as a new press. `press` of `None` keeps the input down until the
/// clicker is stopped, which is what a CPS of zero asks for. Returns whether the
/// clicker may keep going, always with the input released.
fn finish_hold(automator: &mut Automator, scheduler: &Scheduler, press: Option<Duration>) -> bool {
    let pressed = match press {
        Some(press) => scheduler.wait_or_stop(press),
        None => scheduler.wait_until_stopped(),
    };
    automator.end_hold();
    pressed && scheduler.wait_or_stop(scheduler::HOLD_RELEASE_GAP)
}

pub fn run(app: AppHandle, state: Arc<AppState>, rx: Receiver<ClickerSig>) {
    let mut automator = match Automator::new(&app) {
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

            if mode == MODE_NORMAL {
                let target = state.target.lock().unwrap().clone();
                let cps = *state.cps.lock().unwrap();

                let holding = target.click_type == ClickType::Hold;
                let start = Instant::now();
                if holding {
                    automator.begin_hold(&target);
                } else {
                    automator.handle_click(&target);
                }
                let elapsed = start.elapsed();

                if state.is_limited.load(Ordering::SeqCst) {
                    let count = state.num_clicks.fetch_add(1, Ordering::SeqCst) + 1;
                    if count >= state.click_limit.load(Ordering::SeqCst) {
                        if holding {
                            automator.end_hold();
                        }
                        state.stop_and_emit(&app);
                        last_emit = Instant::now();
                        continue;
                    }

                    if last_emit.elapsed() >= EMIT_INTERVAL {
                        state.emit(&app);
                        last_emit = Instant::now();
                    }
                }

                let keep_going = if holding {
                    finish_hold(&mut automator, &scheduler, scheduler::hold_duration(cps))
                } else {
                    scheduler.wait_for_next_cycle(cps, &target, elapsed)
                };

                if !keep_going {
                    state.stop_and_emit(&app);
                    last_emit = Instant::now();
                }
            } else if mode == MODE_SEQUENCE {
                let sequence = state.sequence.lock().unwrap().clone();
                if sequence.is_empty() {
                    state.stop_and_emit(&app);
                    continue;
                }

                let seq_target = &sequence[sequence_index % sequence.len()];
                let target = &seq_target.target;
                let wait_time = seq_target.wait_time.unwrap_or(1000);
                // A hold step reuses its wait as the hold time. A wait of zero
                // leaves no time to hold, so the step stays a plain click rather
                // than stalling the sequence forever.
                let holding = target.click_type == ClickType::Hold && wait_time > 0;

                let start = Instant::now();
                if holding {
                    automator.begin_hold(target);
                } else {
                    automator.handle_click(target);
                }
                let elapsed = start.elapsed();

                sequence_index += 1;

                if state.is_limited.load(Ordering::SeqCst) {
                    if sequence_index.is_multiple_of(sequence.len()) {
                        let count = state.num_clicks.fetch_add(1, Ordering::SeqCst) + 1;
                        if count >= state.click_limit.load(Ordering::SeqCst) {
                            if holding {
                                automator.end_hold();
                            }
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
                    if holding {
                        automator.end_hold();
                    }
                    state.stop_and_emit(&app);
                    last_emit = Instant::now();
                    continue;
                }

                let wait_duration = Duration::from_millis(wait_time).saturating_sub(elapsed);
                if holding {
                    if !finish_hold(&mut automator, &scheduler, Some(wait_duration)) {
                        state.stop_and_emit(&app);
                        last_emit = Instant::now();
                    }
                } else if !scheduler.wait_or_stop(wait_duration) {
                    state.stop_and_emit(&app);
                    last_emit = Instant::now();
                }
            }
        }
    }
}
