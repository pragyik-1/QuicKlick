use crate::automator::{ClickTarget, ClickType};
use crate::ClickerSig;
use std::sync::mpsc::{Receiver, RecvError, RecvTimeoutError, TryRecvError};
use std::time::Duration;

/// Unpressed time between two holds. It has to be long enough for the target to
/// register the next hold as a new press rather than a continuation of the last
pub const HOLD_RELEASE_GAP: Duration = Duration::from_millis(20);

/// Shortest press a hold may last. Above `1 / HOLD_RELEASE_GAP` the interval has
/// no press time left in it, and a hold that lasted nothing would silently
/// degrade into plain clicking, so the press is floored and the cycle stretches.
const MIN_HOLD_PRESS: Duration = Duration::from_millis(1);

/// How long a hold keeps its input pressed, derived from `cps`. `None` means the
/// input stays down until the clicker is stopped.
pub fn hold_duration(cps: f64) -> Option<Duration> {
    if !cps.is_finite() || cps <= 0.0 {
        return None;
    }
    let interval = Duration::from_secs_f64(1.0 / cps);
    Some(
        interval
            .saturating_sub(HOLD_RELEASE_GAP)
            .max(MIN_HOLD_PRESS),
    )
}

pub struct Scheduler<'a> {
    rx: &'a Receiver<ClickerSig>,
}

impl<'a> Scheduler<'a> {
    pub fn new(rx: &'a Receiver<ClickerSig>) -> Self {
        Self { rx }
    }

    pub fn wait_for_next_cycle(
        &self,
        base_cps: f64,
        target: &ClickTarget,
        elapsed: Duration,
    ) -> bool {
        let cps = self.effective_cps(base_cps, target);
        let total_delay = if cps > 0.0 {
            Duration::from_secs_f64(1.0 / cps)
        } else {
            Duration::from_secs(1)
        };
        let wait = total_delay.saturating_sub(elapsed);

        self.wait_or_stop(wait)
    }

    fn effective_cps(&self, base_cps: f64, target: &ClickTarget) -> f64 {
        if target.click_type != ClickType::Randomized {
            return base_cps;
        }
        let max_var = match target.randomize_amount {
            Some(v) if v > 0 => v,
            _ => return base_cps,
        };

        let range = (max_var * 2) + 1;
        let offset_ms = (rand::random::<u64>() % range) as i64 - max_var as i64;
        let base_delay = 1.0 / base_cps;
        let randomized_delay = (base_delay + offset_ms as f64 / 1000.0).max(0.001);
        1.0 / randomized_delay
    }

    pub fn wait_or_stop(&self, wait: Duration) -> bool {
        if wait.is_zero() {
            !matches!(
                self.rx.try_recv(),
                Ok(ClickerSig::Stop) | Err(TryRecvError::Disconnected)
            )
        } else {
            !matches!(
                self.rx.recv_timeout(wait),
                Ok(ClickerSig::Stop) | Err(RecvTimeoutError::Disconnected)
            )
        }
    }

    /// Blocks until a stop is requested. An unbounded hold would otherwise wait
    /// on a duration that does not exist and never look at the signal again.
    /// Returns whether the clicker may keep going; a dropped sender ends it too.
    pub fn wait_until_stopped(&self) -> bool {
        !matches!(self.rx.recv(), Ok(ClickerSig::Stop) | Err(RecvError))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A CPS of zero is the documented way to hold until stopped, so it must not
    /// turn into a zero-length press or a division by zero.
    #[test]
    fn zero_cps_holds_until_stopped() {
        assert_eq!(hold_duration(0.0), None);
        assert_eq!(hold_duration(-5.0), None);
    }

    /// The release gap comes out of the click interval, so the cycle still lasts
    /// `1 / cps`: 10 CPS is a 100ms cycle of 80ms pressed and 20ms released.
    #[test]
    fn press_is_the_interval_minus_the_release_gap() {
        assert_eq!(hold_duration(10.0), Some(Duration::from_millis(80)));
        assert_eq!(hold_duration(1.0), Some(Duration::from_millis(980)));
    }

    /// Faster than one cycle per release gap there is no press time left in the
    /// interval; the press is floored so a hold stays a hold.
    #[test]
    fn fast_cps_keeps_a_press() {
        assert_eq!(hold_duration(1000.0), Some(MIN_HOLD_PRESS));
    }

    /// `Duration::from_secs_f64` panics on a non-finite value, so a CPS that never
    /// passed IPC validation must not reach it.
    #[test]
    fn non_finite_cps_is_rejected_without_panicking() {
        assert_eq!(hold_duration(f64::NAN), None);
        assert_eq!(hold_duration(f64::INFINITY), None);
        assert_eq!(hold_duration(f64::NEG_INFINITY), None);
    }
}
