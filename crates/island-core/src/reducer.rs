use crate::events::{AppEvent, Effect};
use crate::models::IslandView;
use crate::state::AppState;

pub fn reduce(state: &mut AppState, event: AppEvent) -> Vec<Effect> {
    let mut effects = Vec::new();

    match event {
        AppEvent::Tick(time_info) => {
            state.time = time_info;
        }

        AppEvent::BatteryUpdated(bat) => {
            let just_plugged_in = !state.battery.is_charging && bat.is_charging;
            state.battery = bat;

            // When charger is newly connected, trigger global alert with generation ID!
            if just_plugged_in {
                state.alert_generation += 1;
                let gen_id = state.alert_generation;
                state.alert_kind = crate::models::AlertKind::Charging;
                effects.push(Effect::ScheduleTimer {
                    id: "alert_timeout",
                    duration_ms: 2800,
                    generation: gen_id,
                });
            }

            state.update_active_module();
        }

        AppEvent::AlertTimeout(target_gen) => {
            // Only dismiss if the timeout belongs to the current alert generation
            if state.alert_generation == target_gen {
                state.alert_kind = crate::models::AlertKind::None;
                state.update_active_module();
            }
        }

        AppEvent::ToggleSwapPetArt => {
            state.music_visual_mode = match state.music_visual_mode {
                crate::models::MusicVisualMode::PetPrimary => crate::models::MusicVisualMode::ArtworkPrimary,
                crate::models::MusicVisualMode::ArtworkPrimary => crate::models::MusicVisualMode::PetPrimary,
            };
        }

        AppEvent::MediaUpdated(track) => {
            state.media = track;
            state.update_active_module();
        }

        AppEvent::MediaSourcesUpdated(sources) => {
            state.media_sources = sources;
        }

        AppEvent::ToggleSourceSelector => {
            state.show_source_selector = !state.show_source_selector;
            if state.show_source_selector {
                state.active_module = crate::models::ActiveModule::Media;
            }
        }

        AppEvent::SelectMediaSource(id) => {
            for source in &mut state.media_sources {
                source.is_active = source.id == id;
            }
            effects.push(Effect::SelectMediaSource(id));
        }

        AppEvent::SpectrumUpdated(bars) => {
            state.spectrum.bars = bars;
        }

        AppEvent::WindowOverlapChanged(overlapping) => {
            state.engine.handle_overlap(overlapping);
            state.update_active_module();

            if !overlapping && state.is_hovered {
                effects.push(Effect::ScheduleTimer {
                    id: "bloom_wide",
                    duration_ms: 300,
                    generation: 0,
                });
            }
        }

        AppEvent::UserHoverChanged(hovered) => {
            state.engine.handle_hover(hovered);
            state.update_active_module();
        }

        AppEvent::ViewSwitchRequested(target) => {
            state.engine.set_view(target);
            state.update_active_module();
        }

        AppEvent::ModuleSwitchRequested(module) => {
            state.engine.active_module = module;
            state.sync_engine();
        }

        AppEvent::WelcomeTimeout => {
            // Settle into CompactView if not hovered
            if !state.engine.is_hovered {
                state.engine.set_view(IslandView::Compact);
            }
            state.update_active_module();
        }

        AppEvent::IslandClicked => {
            state.engine.handle_click();
            state.update_active_module();
        }

        AppEvent::CollapseClicked => {
            state.engine.handle_collapse();
            state.update_active_module();
        }

        AppEvent::PlayPauseClicked => {
            effects.push(Effect::PlayPause);
        }

        AppEvent::NextTrackClicked => {
            effects.push(Effect::NextTrack);
        }

        AppEvent::PreviousTrackClicked => {
            effects.push(Effect::PreviousTrack);
        }

        AppEvent::DndUpdated(is_dnd) => {
            state.is_dnd = is_dnd;
        }

        AppEvent::AudioMuteUpdated(is_muted) => {
            state.is_muted = is_muted;
        }

        AppEvent::MuteToggleClicked => {
            state.is_muted = !state.is_muted;
            effects.push(Effect::ToggleMute);
        }

        AppEvent::DndToggleClicked => {
            state.is_dnd = !state.is_dnd;
            effects.push(Effect::ToggleDnd);
        }
    }

    effects
}
