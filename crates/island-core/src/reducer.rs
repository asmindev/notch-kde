use crate::events::{AppEvent, Effect};
use crate::models::IslandView;
use crate::prayer::{PrayerAlertKind, PrayerSettings};
use crate::settings::{AvatarSettings, PetBehaviorSettings, ShadowSettings};
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
            if state.mochi_mood == crate::state::MochiMoodState::Idle
                || state.mochi_mood == crate::state::MochiMoodState::Dancing
                || state.mochi_mood == crate::state::MochiMoodState::Sleeping
            {
                if state.pet_is_sleepy {
                    state.mochi_mood = crate::state::MochiMoodState::Sleeping;
                } else if state.media.is_playing {
                    state.mochi_mood = crate::state::MochiMoodState::Dancing;
                } else {
                    state.mochi_mood = crate::state::MochiMoodState::Idle;
                }
            }

            // When charger is newly connected, trigger global alert with generation ID!
            if just_plugged_in {
                state.alert_generation += 1;
                let gen_id = state.alert_generation;
                state.alert_kind = crate::models::AlertKind::Charging;
                state.prayer_alert = None;
                effects.push(Effect::ScheduleTimer {
                    id: "alert_timeout",
                    duration_ms: 2800,
                    generation: gen_id,
                });
            }

            state.update_active_module();
        }

        AppEvent::NotificationReceived => {
            state.notification_generation = state.notification_generation.wrapping_add(1);
        }

        AppEvent::PrayerAlert(alert) => {
            let duration_ms = match alert.kind {
                PrayerAlertKind::Adhan => 9000,
                PrayerAlertKind::Reminder => 7000,
            };
            state.alert_generation += 1;
            let gen_id = state.alert_generation;
            state.alert_kind = crate::models::AlertKind::Prayer;
            state.prayer_alert = Some(alert);
            state.update_active_module();
            effects.push(Effect::ScheduleTimer {
                id: "alert_timeout",
                duration_ms,
                generation: gen_id,
            });
        }

        AppEvent::AlertTimeout(target_gen) => {
            // Only dismiss if the timeout belongs to the current alert generation
            if state.alert_generation == target_gen {
                state.alert_kind = crate::models::AlertKind::None;
                state.prayer_alert = None;
                state.update_active_module();
            }
        }

        AppEvent::CycleBlobCharacter => {
            state.blob_character = (state.blob_character + 1) % 8;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetBlobCharacter(ch) => {
            state.blob_character = ch % 8;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::CycleBlobPalette => {
            state.blob_palette = (state.blob_palette + 1) % 14;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetBlobPalette(palette) => {
            state.blob_palette = palette % 14;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::CycleBlobExpression => {
            state.blob_expression = (state.blob_expression + 1) % 17;
            state.blob_expression_custom = true;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetBlobExpression(expression) => {
            state.blob_expression = expression % 17;
            state.blob_expression_custom = true;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::RandomizeAvatar => {
            randomize_avatar(state);
            effects.push(Effect::PersistSettings);
        }

        AppEvent::AvatarRandomTick => {
            if state.avatar_random {
                randomize_avatar(state);
            }
        }

        AppEvent::ToggleAvatarRandom => {
            state.avatar_random = !state.avatar_random;
            if state.avatar_random {
                randomize_avatar(state);
            }
            effects.push(Effect::PersistSettings);
        }

        AppEvent::ToggleSwapPetArt => {
            state.music_visual_mode = match state.music_visual_mode {
                crate::models::MusicVisualMode::PetPrimary => crate::models::MusicVisualMode::ArtworkPrimary,
                crate::models::MusicVisualMode::ArtworkPrimary => crate::models::MusicVisualMode::PetPrimary,
            };
            effects.push(Effect::PersistSettings);
        }

        AppEvent::PetClicked | AppEvent::MochiSlapped => {
            if !state.behavior.click_stunts {
                return effects;
            }
            state.mochi_slap_count += 1;
            state.mochi_generation += 1;
            state.pet_poke_count = state.mochi_slap_count;
            state.pet_happy_generation = state.mochi_generation;
            let gen_id = state.mochi_generation;

            // Coucou slap logic: 3+ slaps within 1.7s triggers dizzy
            if state.mochi_slap_count >= 3 {
                state.mochi_mood = crate::state::MochiMoodState::Dizzy;
                state.pet_is_dizzy = true;
                state.pet_is_happy = false;
                state.pet_dizzy_generation = gen_id;
                effects.push(Effect::ScheduleTimer {
                    id: "mochi_dizzy_timeout",
                    duration_ms: 2200,
                    generation: gen_id,
                });
                effects.push(Effect::ScheduleTimer {
                    id: "pet_dizzy_timeout",
                    duration_ms: 2200,
                    generation: gen_id,
                });
            } else {
                // 1st or 2nd slap: Annoyed line eyes for 800ms
                state.mochi_mood = crate::state::MochiMoodState::Annoyed;
                state.pet_is_happy = true;
                state.pet_is_dizzy = false;
                effects.push(Effect::ScheduleTimer {
                    id: "mochi_annoyed_timeout",
                    duration_ms: 800,
                    generation: gen_id,
                });
                effects.push(Effect::ScheduleTimer {
                    id: "pet_happy_timeout",
                    duration_ms: 1500,
                    generation: gen_id,
                });
            }

            effects.push(Effect::ScheduleTimer {
                id: "mochi_slap_reset",
                duration_ms: 1700,
                generation: gen_id,
            });
            effects.push(Effect::ScheduleTimer {
                id: "pet_poke_reset",
                duration_ms: 1700,
                generation: gen_id,
            });
        }

        AppEvent::MochiAnnoyedTimeout(target_gen) => {
            if state.mochi_generation == target_gen && state.mochi_mood == crate::state::MochiMoodState::Annoyed {
                state.mochi_mood = if state.media.is_playing {
                    crate::state::MochiMoodState::Dancing
                } else if state.battery.percentage <= 20 && !state.battery.is_charging {
                    crate::state::MochiMoodState::Sleeping
                } else {
                    crate::state::MochiMoodState::Idle
                };
            }
        }

        AppEvent::MochiDizzyTimeout(target_gen) | AppEvent::PetDizzyTimeout(target_gen) => {
            if state.mochi_generation == target_gen || state.pet_dizzy_generation == target_gen {
                state.mochi_mood = if state.media.is_playing {
                    crate::state::MochiMoodState::Dancing
                } else if state.battery.percentage <= 20 && !state.battery.is_charging {
                    crate::state::MochiMoodState::Sleeping
                } else {
                    crate::state::MochiMoodState::Idle
                };
                state.pet_is_dizzy = false;
                state.mochi_slap_count = 0;
                state.pet_poke_count = 0;
            }
        }

        AppEvent::MochiHappyTimeout(target_gen) | AppEvent::PetHappyTimeout(target_gen) => {
            if state.mochi_generation == target_gen || state.pet_happy_generation == target_gen {
                if state.mochi_mood == crate::state::MochiMoodState::Happy {
                    state.mochi_mood = if state.media.is_playing {
                        crate::state::MochiMoodState::Dancing
                    } else if state.battery.percentage <= 20 && !state.battery.is_charging {
                        crate::state::MochiMoodState::Sleeping
                    } else {
                        crate::state::MochiMoodState::Idle
                    };
                }
                state.pet_is_happy = false;
            }
        }

        AppEvent::MochiSlapReset(target_gen) | AppEvent::PetPokeReset(target_gen) => {
            if state.mochi_generation == target_gen || state.pet_happy_generation == target_gen {
                state.mochi_slap_count = 0;
                state.pet_poke_count = 0;
            }
        }

        AppEvent::MediaUpdated(track) => {
            state.media = track;
            state.update_active_module();
            if state.mochi_mood == crate::state::MochiMoodState::Idle
                || state.mochi_mood == crate::state::MochiMoodState::Dancing
                || state.mochi_mood == crate::state::MochiMoodState::Sleeping
            {
                if state.media.is_playing {
                    state.mochi_mood = crate::state::MochiMoodState::Dancing;
                } else if state.battery.percentage <= 20 && !state.battery.is_charging {
                    state.mochi_mood = crate::state::MochiMoodState::Sleeping;
                } else {
                    state.mochi_mood = crate::state::MochiMoodState::Idle;
                }
            }
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

        AppEvent::OpenSettings => {
            state.settings_open = true;
        }

        AppEvent::CloseSettings => {
            state.settings_open = false;
        }

        AppEvent::SetSettingsTab(tab) => {
            state.settings_tab = tab;
        }

        AppEvent::SetCursorGaze(enabled) => {
            state.behavior.cursor_gaze = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetIdleActing(enabled) => {
            state.behavior.idle_acting = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetClickStunts(enabled) => {
            state.behavior.click_stunts = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetNotificationReactions(enabled) => {
            state.behavior.notification_reactions = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetMusicReactions(enabled) => {
            state.behavior.music_reactions = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetSleepyOnLowBattery(enabled) => {
            state.behavior.sleepy_on_low_battery = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetShadowEnabled(enabled) => {
            state.shadow.enabled = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetShadowIntensity(intensity) => {
            state.shadow.intensity = intensity.min(crate::settings::SHADOW_INTENSITY_MAX);
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetShadowRadius(radius) => {
            state.shadow.radius = radius.min(crate::settings::SHADOW_RADIUS_MAX);
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetShadowOffsetX(offset) => {
            state.shadow.offset_x = offset.clamp(
                -crate::settings::SHADOW_OFFSET_MAX,
                crate::settings::SHADOW_OFFSET_MAX,
            );
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetShadowOffsetY(offset) => {
            state.shadow.offset_y = offset.clamp(
                -crate::settings::SHADOW_OFFSET_MAX,
                crate::settings::SHADOW_OFFSET_MAX,
            );
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetPrayerEnabled(enabled) => {
            state.prayer.enabled = enabled;
            state.prayer = state.prayer.normalized();
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetPrayerCity(city) => {
            state.prayer.city = city;
            state.prayer = state.prayer.normalized();
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetPrayerReminderMinutes(minutes) => {
            state.prayer.reminder_minutes = minutes;
            state.prayer = state.prayer.normalized();
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetMediaVisualMode(mode) => {
            state.music_visual_mode = mode;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::ResetAvatar => {
            let avatar = AvatarSettings::default();
            state.blob_character = avatar.character;
            state.blob_palette = avatar.palette;
            state.blob_expression = avatar.expression;
            state.blob_expression_custom = avatar.expression_custom;
            state.avatar_random = avatar.random;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::ResetSettings => {
            let avatar = AvatarSettings::default();
            state.blob_character = avatar.character;
            state.blob_palette = avatar.palette;
            state.blob_expression = avatar.expression;
            state.blob_expression_custom = avatar.expression_custom;
            state.avatar_random = avatar.random;
            state.behavior = PetBehaviorSettings::default();
            state.shadow = ShadowSettings::default();
            state.prayer = PrayerSettings::default();
            state.music_visual_mode = crate::models::MusicVisualMode::ArtworkPrimary;
            effects.push(Effect::PersistSettings);
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

fn next_avatar_seed(state: &mut AppState) -> u64 {
    let mut x = state.avatar_seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    state.avatar_seed = x;
    x
}

fn randomize_avatar(state: &mut AppState) {
    state.blob_character = (next_avatar_seed(state) % 8) as u32;
    state.blob_palette = (next_avatar_seed(state) % 14) as u32;
    state.blob_expression = (next_avatar_seed(state) % 17) as u32;
    state.blob_expression_custom = true;
}
