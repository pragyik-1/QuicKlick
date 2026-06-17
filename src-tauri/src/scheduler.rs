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
        let mut final_cps = base_cps;
        if target.click_type == ClickType::Randomized {
            if let Some(max_variation) = target.randomize_amount {
                if max_variation > 0 {
                    let range = (max_variation * 2) + 1;
                    let offset_ms = (rand::random::<u64>() % range) as i64 - max_variation as i64;
                    let base_delay = 1.0 / base_cps;
                    let randomized_delay = base_delay + (offset_ms as f64 / 1000.0);
                    final_cps = 1.0 / randomized_delay.max(0.001);
                }
            }
        }
        let total_delay = if final_cps > 0.0 {
            Duration::from_secs_f64(1.0 / final_cps)
        } else {
            Duration::from_secs(1)
        };
        let wait_time = total_delay.saturating_sub(elapsed);

        if wait_time.is_zero() {
            match self.rx.try_recv() {
                Ok(ClickerSig::Stop) => false,
                Ok(ClickerSig::Start) | Err(TryRecvError::Empty) => true,
                Err(TryRecvError::Disconnected) => false,
            }
        } else {
            match self.rx.recv_timeout(wait_time) {
                Ok(ClickerSig::Stop) => false,
                Ok(ClickerSig::Start) | Err(RecvTimeoutError::Timeout) => true,
                Err(RecvTimeoutError::Disconnected) => false,
            }
        }
    }
}
