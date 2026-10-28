use crate::events::{AppEvent, Effect};
use crate::models::IslandView;
use crate::state::AppState;

fn begin_overlap_dismissal(state: &mut AppState, effects: &mut Vec<Effect>) {
    state.overlap_reveal_generation += 1;
    state.overlap_dismiss_generation += 1;
    let generation = state.overlap_dismiss_generation;
    state.engine.begin_overlap_dismiss();
    state.update_active_module();
    effects.push(Effect::ScheduleTimer {
        id: "overlap_dismiss_compact",
        duration_ms: 250,
        generation,
    });
}

fn begin_standard_collapse(state: &mut AppState, effects: &mut Vec<Effect>) {
    state.overlap_reveal_generation += 1;
    state.overlap_dismiss_generation += 1;
    let generation = state.overlap_dismiss_generation;
    state.engine.begin_overlap_dismiss();
    state.update_active_module();
    effects.push(Effect::ScheduleTimer {
        id: "collapse_compact",
        duration_ms: 250,
        generation,
    });
}

pub fn reduce(state: &mut AppState, event: AppEvent) -> Vec<Effect> {
    let mut effects = Vec::new();

    match event {
        AppEvent::Tick(time_info) => {
            state.time = time_info;
        }

        AppEvent::BatteryUpdated(bat) => {
            let just_plugged_in = !state.battery.is_charging && bat.is_charging;
            state.battery = bat;
            state.pet_is_sleepy = state.battery.percentage <= 20 && !state.battery.is_charging;

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

        AppEvent::PetClicked => {
            state.pet_poke_count += 1;
            state.pet_happy_generation += 1;
            let gen_id = state.pet_happy_generation;

            if state.pet_poke_count >= 4 {
                state.pet_is_dizzy = true;
                state.pet_is_happy = false;
                state.pet_dizzy_generation += 1;
                let dizzy_gen = state.pet_dizzy_generation;
                effects.push(Effect::ScheduleTimer {
                    id: "pet_dizzy_timeout",
                    duration_ms: 2200,
                    generation: dizzy_gen,
                });
            } else {
                state.pet_is_happy = true;
                effects.push(Effect::ScheduleTimer {
                    id: "pet_happy_timeout",
                    duration_ms: 1500,
                    generation: gen_id,
                });
            }

            effects.push(Effect::ScheduleTimer {
                id: "pet_poke_reset",
                duration_ms: 1200,
                generation: gen_id,
            });
        }

        AppEvent::PetHappyTimeout(target_gen) => {
            if state.pet_happy_generation == target_gen {
                state.pet_is_happy = false;
            }
        }

        AppEvent::PetDizzyTimeout(target_gen) => {
            if state.pet_dizzy_generation == target_gen {
                state.pet_is_dizzy = false;
                state.pet_poke_count = 0;
            }
        }

        AppEvent::PetPokeReset(target_gen) => {
            if state.pet_happy_generation == target_gen {
                state.pet_poke_count = 0;
            }
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
            if !overlapping {
                state.overlap_dismiss_generation += 1;
            }
            state.engine.handle_overlap(overlapping);
            state.update_active_module();
        }

        AppEvent::UserHoverChanged(hovered) => {
            if !hovered
                && !state.engine.overlap_dismiss_active
                && state.engine.current_view == IslandView::Wide
                && state.engine.is_overlapping
            {
                state.overlap_reveal_generation += 1;
                state.overlap_dismiss_generation += 1;
                let generation = state.overlap_dismiss_generation;
                state.engine.begin_hover_overlap_dismiss();
                state.update_active_module();
                effects.push(Effect::ScheduleTimer {
                    id: "overlap_dismiss_hide",
                    duration_ms: 250,
                    generation,
                });
            } else {
                let was_hidden = !state.is_visible;
                let was_dismissing = state.engine.overlap_dismiss_active;
                let had_reveal_hold = state.engine.overlap_reveal_hold;

                state.engine.handle_hover(hovered);
                state.update_active_module();

                if was_dismissing && !state.engine.overlap_dismiss_active {
                    state.overlap_dismiss_generation += 1;
                }

                if !hovered {
                    state.overlap_reveal_generation += 1;
                } else if was_hidden && state.engine.overlap_reveal_hold && !had_reveal_hold {
                    state.overlap_reveal_generation += 1;
                    effects.push(Effect::ScheduleTimer {
                        id: "overlap_reveal",
                        duration_ms: 300,
                        generation: state.overlap_reveal_generation,
                    });
                }
            }
        }

        AppEvent::OverlapRevealTimeout(generation) => {
            if generation == state.overlap_reveal_generation {
                state.engine.finish_overlap_reveal();
                state.update_active_module();
            }
        }

        AppEvent::OverlapDismissCompactTimeout(generation) => {
            if generation == state.overlap_dismiss_generation
                && state.engine.overlap_dismiss_active
                && state.engine.is_overlapping
            {
                state.engine.advance_overlap_dismiss();
                state.update_active_module();
                effects.push(Effect::ScheduleTimer {
                    id: "overlap_dismiss_hide",
                    duration_ms: 250,
                    generation,
                });
            }
        }

        AppEvent::OverlapDismissHideTimeout(generation) => {
            if generation == state.overlap_dismiss_generation
                && state.engine.overlap_dismiss_active
                && state.engine.is_overlapping
            {
                state.engine.finish_overlap_dismiss();
                state.update_active_module();
            }
        }

        AppEvent::CollapseCompactTimeout(generation) => {
            if generation == state.overlap_dismiss_generation
                && state.engine.overlap_dismiss_active
                && !state.engine.is_overlapping
            {
                state.engine.advance_overlap_dismiss();
                state.engine.finish_overlap_dismiss();
                state.update_active_module();
            }
        }

        AppEvent::ViewSwitchRequested(target) => {
            state.overlap_reveal_generation += 1;
            state.overlap_dismiss_generation += 1;
            state.engine.cancel_overlap_sequences();
            state.engine.set_view(target);
            state.update_active_module();
        }

        AppEvent::ModuleSwitchRequested(module) => {
            state.engine.active_module = module;
            state.sync_engine();
        }

        AppEvent::WelcomeTimeout => {
            state.is_welcome = false;
            // Settle into CompactView if not hovered
            if !state.engine.is_hovered {
                state.engine.set_view(IslandView::Compact);
            }
            state.update_active_module();
        }

        AppEvent::ScreenUnlocked | AppEvent::TriggerWelcome => {
            state.is_welcome = true;
            state.pet_is_happy = true;
            state.pet_happy_generation += 1;
            let happy_gen = state.pet_happy_generation;
            state.welcome_generation += 1;
            let gen_id = state.welcome_generation;
            state.overlap_reveal_generation += 1;
            state.overlap_dismiss_generation += 1;
            state.engine.cancel_overlap_sequences();
            state.engine.set_view(IslandView::Wide);
            state.update_active_module();
            effects.push(Effect::ScheduleTimer {
                id: "pet_happy_timeout",
                duration_ms: 1500,
                generation: happy_gen,
            });
            effects.push(Effect::ScheduleTimer {
                id: "welcome_timeout",
                duration_ms: 3500,
                generation: gen_id,
            });
        }

        AppEvent::IslandClicked => {
            if !state.is_visible && state.engine.is_overlapping {
                state.overlap_reveal_generation += 1;
                state.overlap_dismiss_generation += 1;
                state.engine.reveal_overlap_compact();
                state.update_active_module();
            } else if state.engine.current_view == IslandView::Expand {
                if state.engine.is_overlapping {
                    begin_overlap_dismissal(state, &mut effects);
                } else {
                    begin_standard_collapse(state, &mut effects);
                }
            } else {
                state.overlap_reveal_generation += 1;
                state.overlap_dismiss_generation += 1;
                state.engine.cancel_overlap_sequences();
                state.engine.handle_click();
                state.update_active_module();
            }
        }

        AppEvent::CollapseClicked => {
            if state.engine.current_view == IslandView::Expand {
                if state.engine.is_overlapping {
                    begin_overlap_dismissal(state, &mut effects);
                } else {
                    begin_standard_collapse(state, &mut effects);
                }
            } else {
                state.overlap_reveal_generation += 1;
                state.overlap_dismiss_generation += 1;
                state.engine.cancel_overlap_sequences();
                state.engine.handle_collapse();
                state.update_active_module();
            }
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
