use crate::automator::{ClickTarget, ClickType};
use crate::ClickerSig;
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::time::Duration;

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

    fn wait_or_stop(&self, wait: Duration) -> bool {
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
}
