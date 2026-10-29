use crate::runtime::timers::TimerScheduler;
use crate::settings_store::SettingsStore;
use island_core::{AppEvent, AppState, Effect, PersistedSettings};
use island_services::{AudioService, DndService, MediaService};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

pub struct EffectExecutor {
    runtime: Handle,
    timers: TimerScheduler,
}

impl EffectExecutor {
    pub fn new(runtime: Handle, event_tx: UnboundedSender<AppEvent>) -> Self {
        Self {
            timers: TimerScheduler::new(runtime.clone(), event_tx),
            runtime,
        }
    }

    pub fn execute(&mut self, effect: Effect, state: &AppState) {
        match effect {
            Effect::PlayPause => {
                self.runtime.spawn(async {
                    MediaService::play_pause().await;
                });
            }
            Effect::NextTrack => {
                self.runtime.spawn(async {
                    MediaService::next().await;
                });
            }
            Effect::PreviousTrack => {
                self.runtime.spawn(async {
                    MediaService::previous().await;
                });
            }
            Effect::ToggleMute => {
                debug!("Toggling system mute");
                self.runtime.spawn(async {
                    AudioService::toggle_mute().await;
                });
            }
            Effect::ToggleDnd => {
                debug!("Toggling notification Do-Not-Disturb");
                self.runtime.spawn(async {
                    DndService::toggle_dnd().await;
                });
            }
            Effect::PersistSettings => {
                SettingsStore::save(&PersistedSettings::from_state(state));
            }
            Effect::SelectMediaSource(player_id) => {
                debug!("Switching active media source to: {}", player_id);
                let runtime = self.runtime.clone();
                runtime.spawn(async move {
                    MediaService::select_source(&player_id).await;
                });
            }
            Effect::ScheduleTimer {
                id,
                duration_ms,
                generation,
            } => {
                self.timers
                    .schedule(id, Duration::from_millis(duration_ms), generation);
            }
        }
    }

    pub fn shutdown(&mut self) {
        self.timers.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use island_core::TimerId;
    use std::collections::HashSet;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn timer_effects_are_forwarded_to_the_scheduler() {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let mut executor = EffectExecutor::new(Handle::current(), event_tx);
        let state = AppState::default();
        let ids = [
            TimerId::Alert,
            TimerId::CollapseCompact,
            TimerId::MochiAnnoyed,
            TimerId::MochiDizzy,
            TimerId::MochiSlapReset,
            TimerId::OverlapDismissCompact,
            TimerId::OverlapDismissHide,
            TimerId::OverlapReveal,
            TimerId::PetDizzy,
            TimerId::PetHappy,
            TimerId::PetPokeReset,
            TimerId::Welcome,
        ];

        for id in ids {
            executor.execute(
                Effect::ScheduleTimer {
                    id,
                    duration_ms: 1,
                    generation: 9,
                },
                &state,
            );
        }

        let mut fired = HashSet::new();
        for _ in ids {
            let event = tokio::time::timeout(Duration::from_secs(1), event_rx.recv())
                .await
                .expect("timer did not fire")
                .expect("event channel closed");
            match event {
                AppEvent::TimerFired { id, generation } => {
                    assert_eq!(generation, 9);
                    fired.insert(id);
                }
                other => panic!("unexpected event: {other:?}"),
            }
        }
        assert_eq!(fired.len(), ids.len());
        executor.shutdown();
    }
}
