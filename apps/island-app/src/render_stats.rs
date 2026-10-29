use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderSnapshot {
    pub pet_ticks: u64,
    pub pet_redraws: u64,
    pub ui_syncs: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderRates {
    pub pet_ticks_per_sec: f64,
    pub pet_redraws_per_sec: f64,
    pub ui_syncs_per_sec: f64,
}

impl RenderSnapshot {
    pub fn since(self, previous: Self) -> Self {
        Self {
            pet_ticks: self.pet_ticks.saturating_sub(previous.pet_ticks),
            pet_redraws: self.pet_redraws.saturating_sub(previous.pet_redraws),
            ui_syncs: self.ui_syncs.saturating_sub(previous.ui_syncs),
        }
    }

    pub fn rates_since(self, previous: Self, elapsed_secs: f64) -> RenderRates {
        let delta = self.since(previous);
        let elapsed = elapsed_secs.max(f64::EPSILON);
        RenderRates {
            pet_ticks_per_sec: delta.pet_ticks as f64 / elapsed,
            pet_redraws_per_sec: delta.pet_redraws as f64 / elapsed,
            ui_syncs_per_sec: delta.ui_syncs as f64 / elapsed,
        }
    }
}

#[derive(Default)]
pub struct RenderStats {
    pet_ticks: AtomicU64,
    pet_redraws: AtomicU64,
    ui_syncs: AtomicU64,
}

impl RenderStats {
    pub fn pet_tick(&self) {
        self.pet_ticks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn pet_redraw(&self) {
        self.pet_redraws.fetch_add(1, Ordering::Relaxed);
    }

    pub fn ui_sync(&self) {
        self.ui_syncs.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> RenderSnapshot {
        RenderSnapshot {
            pet_ticks: self.pet_ticks.load(Ordering::Relaxed),
            pet_redraws: self.pet_redraws.load(Ordering::Relaxed),
            ui_syncs: self.ui_syncs.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_use_counter_deltas() {
        let stats = RenderStats::default();
        let start = stats.snapshot();
        for _ in 0..4 {
            stats.pet_tick();
        }
        for _ in 0..2 {
            stats.pet_redraw();
        }
        for _ in 0..6 {
            stats.ui_sync();
        }

        let rates = stats.snapshot().rates_since(start, 2.0);

        assert_eq!(rates.pet_ticks_per_sec, 2.0);
        assert_eq!(rates.pet_redraws_per_sec, 1.0);
        assert_eq!(rates.ui_syncs_per_sec, 3.0);
    }
}
