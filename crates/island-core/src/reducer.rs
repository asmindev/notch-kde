use crate::events::{AppEvent, Effect, TimerId};
use crate::models::IslandView;
use crate::prayer::{PrayerAlertKind, PrayerSettings};
use crate::settings::{AvatarSettings, PetBehaviorSettings, ShadowSettings};
use crate::state::AppState;

fn begin_overlap_dismissal(state: &mut AppState, effects: &mut Vec<Effect>) {
    state.runtime.overlap_reveal_generation += 1;
    state.runtime.overlap_dismiss_generation += 1;
    let generation = state.runtime.overlap_dismiss_generation;
    state.view.engine.begin_overlap_dismiss();
    state.update_active_module();
    effects.push(Effect::ScheduleTimer {
        id: TimerId::OverlapDismissCompact,
        duration_ms: 250,
        generation,
    });
}

fn begin_standard_collapse(state: &mut AppState, effects: &mut Vec<Effect>) {
    state.runtime.overlap_reveal_generation += 1;
    state.runtime.overlap_dismiss_generation += 1;
    let generation = state.runtime.overlap_dismiss_generation;
    state.view.engine.begin_overlap_dismiss();
    state.update_active_module();
    effects.push(Effect::ScheduleTimer {
        id: TimerId::CollapseCompact,
        duration_ms: 250,
        generation,
    });
}

pub fn reduce(state: &mut AppState, event: AppEvent) -> Vec<Effect> {
    let mut effects = Vec::new();

    match event {
        AppEvent::Tick(time_info) => {
            state.domain.time = time_info;
        }

        AppEvent::BatteryUpdated(bat) => {
            let just_plugged_in = !state.domain.battery.is_charging && bat.is_charging;
            state.domain.battery = bat;
            state.presentation.pet_is_sleepy =
                state.domain.battery.percentage <= 20 && !state.domain.battery.is_charging;
            if state.presentation.mochi_mood == crate::state::MochiMoodState::Idle
                || state.presentation.mochi_mood == crate::state::MochiMoodState::Dancing
                || state.presentation.mochi_mood == crate::state::MochiMoodState::Sleeping
            {
                if state.presentation.pet_is_sleepy {
                    state.presentation.mochi_mood = crate::state::MochiMoodState::Sleeping;
                } else if state.domain.media.is_playing {
                    state.presentation.mochi_mood = crate::state::MochiMoodState::Dancing;
                } else {
                    state.presentation.mochi_mood = crate::state::MochiMoodState::Idle;
                }
            }

            // When charger is newly connected, trigger global alert with generation ID!
            if just_plugged_in {
                state.domain.alert_generation += 1;
                let gen_id = state.domain.alert_generation;
                state.domain.alert_kind = crate::models::AlertKind::Charging;
                state.domain.prayer_alert = None;
                effects.push(Effect::ScheduleTimer {
                    id: TimerId::Alert,
                    duration_ms: 2800,
                    generation: gen_id,
                });
            }

            state.update_active_module();
        }

        AppEvent::NotificationReceived => {
            state.presentation.notification_generation =
                state.presentation.notification_generation.wrapping_add(1);
        }

        AppEvent::PrayerAlert(alert) => {
            let duration_ms = match alert.kind {
                PrayerAlertKind::Adhan => 9000,
                PrayerAlertKind::Reminder => 7000,
            };
            state.domain.alert_generation += 1;
            let gen_id = state.domain.alert_generation;
            state.domain.alert_kind = crate::models::AlertKind::Prayer;
            state.domain.prayer_alert = Some(alert);
            state.update_active_module();
            effects.push(Effect::ScheduleTimer {
                id: TimerId::Alert,
                duration_ms,
                generation: gen_id,
            });
        }

        AppEvent::TimerFired {
            id: TimerId::Alert,
            generation: target_gen,
        } => {
            // Only dismiss if the timeout belongs to the current alert generation
            if state.domain.alert_generation == target_gen {
                state.domain.alert_kind = crate::models::AlertKind::None;
                state.domain.prayer_alert = None;
                state.update_active_module();
            }
        }

        AppEvent::CycleBlobCharacter => {
            state.settings.blob_character = (state.settings.blob_character + 1) % 8;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetBlobCharacter(ch) => {
            state.settings.blob_character = ch % 8;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::CycleBlobPalette => {
            state.settings.blob_palette = (state.settings.blob_palette + 1) % 14;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetBlobPalette(palette) => {
            state.settings.blob_palette = palette % 14;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::CycleBlobExpression => {
            state.settings.blob_expression = (state.settings.blob_expression + 1) % 17;
            state.settings.blob_expression_custom = true;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetBlobExpression(expression) => {
            state.settings.blob_expression = expression % 17;
            state.settings.blob_expression_custom = true;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::RandomizeAvatar => {
            randomize_avatar(state);
            effects.push(Effect::PersistSettings);
        }

        AppEvent::AvatarRandomTick => {
            if state.settings.avatar_random {
                randomize_avatar(state);
            }
        }

        AppEvent::ToggleAvatarRandom => {
            state.settings.avatar_random = !state.settings.avatar_random;
            if state.settings.avatar_random {
                randomize_avatar(state);
            }
            effects.push(Effect::PersistSettings);
        }

        AppEvent::ToggleSwapPetArt => {
            state.settings.music_visual_mode = match state.settings.music_visual_mode {
                crate::models::MusicVisualMode::PetPrimary => {
                    crate::models::MusicVisualMode::ArtworkPrimary
                }
                crate::models::MusicVisualMode::ArtworkPrimary => {
                    crate::models::MusicVisualMode::PetPrimary
                }
            };
            effects.push(Effect::PersistSettings);
        }

        AppEvent::PetClicked | AppEvent::MochiSlapped => {
            if !state.settings.behavior.click_stunts {
                return effects;
            }
            state.presentation.mochi_slap_count += 1;
            state.presentation.mochi_generation += 1;
            state.presentation.pet_poke_count = state.presentation.mochi_slap_count;
            state.presentation.pet_happy_generation = state.presentation.mochi_generation;
            let gen_id = state.presentation.mochi_generation;

            // Coucou slap logic: 3+ slaps within 1.7s triggers dizzy
            if state.presentation.mochi_slap_count >= 3 {
                state.presentation.mochi_mood = crate::state::MochiMoodState::Dizzy;
                state.presentation.pet_is_dizzy = true;
                state.presentation.pet_is_happy = false;
                state.presentation.pet_dizzy_generation = gen_id;
                effects.push(Effect::ScheduleTimer {
                    id: TimerId::MochiDizzy,
                    duration_ms: 2200,
                    generation: gen_id,
                });
                effects.push(Effect::ScheduleTimer {
                    id: TimerId::PetDizzy,
                    duration_ms: 2200,
                    generation: gen_id,
                });
            } else {
                // 1st or 2nd slap: Annoyed line eyes for 800ms
                state.presentation.mochi_mood = crate::state::MochiMoodState::Annoyed;
                state.presentation.pet_is_happy = true;
                state.presentation.pet_is_dizzy = false;
                effects.push(Effect::ScheduleTimer {
                    id: TimerId::MochiAnnoyed,
                    duration_ms: 800,
                    generation: gen_id,
                });
                effects.push(Effect::ScheduleTimer {
                    id: TimerId::PetHappy,
                    duration_ms: 1500,
                    generation: gen_id,
                });
            }

            effects.push(Effect::ScheduleTimer {
                id: TimerId::MochiSlapReset,
                duration_ms: 1700,
                generation: gen_id,
            });
            effects.push(Effect::ScheduleTimer {
                id: TimerId::PetPokeReset,
                duration_ms: 1700,
                generation: gen_id,
            });
        }

        AppEvent::TimerFired {
            id: TimerId::MochiAnnoyed,
            generation: target_gen,
        } => {
            if state.presentation.mochi_generation == target_gen
                && state.presentation.mochi_mood == crate::state::MochiMoodState::Annoyed
            {
                state.presentation.mochi_mood = if state.domain.media.is_playing {
                    crate::state::MochiMoodState::Dancing
                } else if state.domain.battery.percentage <= 20 && !state.domain.battery.is_charging
                {
                    crate::state::MochiMoodState::Sleeping
                } else {
                    crate::state::MochiMoodState::Idle
                };
            }
        }

        AppEvent::TimerFired {
            id: TimerId::MochiDizzy | TimerId::PetDizzy,
            generation: target_gen,
        } => {
            if state.presentation.mochi_generation == target_gen
                || state.presentation.pet_dizzy_generation == target_gen
            {
                state.presentation.mochi_mood = if state.domain.media.is_playing {
                    crate::state::MochiMoodState::Dancing
                } else if state.domain.battery.percentage <= 20 && !state.domain.battery.is_charging
                {
                    crate::state::MochiMoodState::Sleeping
                } else {
                    crate::state::MochiMoodState::Idle
                };
                state.presentation.pet_is_dizzy = false;
                state.presentation.mochi_slap_count = 0;
                state.presentation.pet_poke_count = 0;
            }
        }

        AppEvent::TimerFired {
            id: TimerId::PetHappy,
            generation: target_gen,
        } => {
            if state.presentation.mochi_generation == target_gen
                || state.presentation.pet_happy_generation == target_gen
            {
                if state.presentation.mochi_mood == crate::state::MochiMoodState::Happy {
                    state.presentation.mochi_mood = if state.domain.media.is_playing {
                        crate::state::MochiMoodState::Dancing
                    } else if state.domain.battery.percentage <= 20
                        && !state.domain.battery.is_charging
                    {
                        crate::state::MochiMoodState::Sleeping
                    } else {
                        crate::state::MochiMoodState::Idle
                    };
                }
                state.presentation.pet_is_happy = false;
            }
        }

        AppEvent::TimerFired {
            id: TimerId::MochiSlapReset | TimerId::PetPokeReset,
            generation: target_gen,
        } => {
            if state.presentation.mochi_generation == target_gen
                || state.presentation.pet_happy_generation == target_gen
            {
                state.presentation.mochi_slap_count = 0;
                state.presentation.pet_poke_count = 0;
            }
        }

        AppEvent::MediaUpdated(track) => {
            state.domain.media = track;
            state.update_active_module();
            if state.presentation.mochi_mood == crate::state::MochiMoodState::Idle
                || state.presentation.mochi_mood == crate::state::MochiMoodState::Dancing
                || state.presentation.mochi_mood == crate::state::MochiMoodState::Sleeping
            {
                if state.domain.media.is_playing {
                    state.presentation.mochi_mood = crate::state::MochiMoodState::Dancing;
                } else if state.domain.battery.percentage <= 20 && !state.domain.battery.is_charging
                {
                    state.presentation.mochi_mood = crate::state::MochiMoodState::Sleeping;
                } else {
                    state.presentation.mochi_mood = crate::state::MochiMoodState::Idle;
                }
            }
        }

        AppEvent::MediaSourcesUpdated(sources) => {
            state.domain.media_sources = sources;
        }

        AppEvent::ToggleSourceSelector => {
            state.module.show_source_selector = !state.module.show_source_selector;
            if state.module.show_source_selector {
                state.module.prayer_panel_open = false;
            }
            state.update_active_module();
        }

        AppEvent::TogglePrayerPanel => {
            state.module.prayer_panel_open = !state.module.prayer_panel_open;
            if state.module.prayer_panel_open {
                state.module.show_source_selector = false;
            }
            state.update_active_module();
        }

        AppEvent::OpenSettings => {
            state.presentation.settings_open = true;
        }

        AppEvent::CloseSettings => {
            state.presentation.settings_open = false;
        }

        AppEvent::SetSettingsTab(tab) => {
            state.presentation.settings_tab = tab;
        }

        AppEvent::SetCursorGaze(enabled) => {
            state.settings.behavior.cursor_gaze = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetIdleActing(enabled) => {
            state.settings.behavior.idle_acting = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetClickStunts(enabled) => {
            state.settings.behavior.click_stunts = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetNotificationReactions(enabled) => {
            state.settings.behavior.notification_reactions = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetMusicReactions(enabled) => {
            state.settings.behavior.music_reactions = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetSleepyOnLowBattery(enabled) => {
            state.settings.behavior.sleepy_on_low_battery = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetShadowEnabled(enabled) => {
            state.settings.shadow.enabled = enabled;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetShadowIntensity(intensity) => {
            state.settings.shadow.intensity = intensity.min(crate::settings::SHADOW_INTENSITY_MAX);
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetShadowRadius(radius) => {
            state.settings.shadow.radius = radius.min(crate::settings::SHADOW_RADIUS_MAX);
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetShadowOffsetX(offset) => {
            state.settings.shadow.offset_x = offset.clamp(
                -crate::settings::SHADOW_OFFSET_MAX,
                crate::settings::SHADOW_OFFSET_MAX,
            );
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetShadowOffsetY(offset) => {
            state.settings.shadow.offset_y = offset.clamp(
                -crate::settings::SHADOW_OFFSET_MAX,
                crate::settings::SHADOW_OFFSET_MAX,
            );
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetPrayerEnabled(enabled) => {
            state.settings.prayer.enabled = enabled;
            state.settings.prayer = state.settings.prayer.normalized();
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetPrayerCity(city) => {
            state.settings.prayer.city = city;
            state.settings.prayer = state.settings.prayer.normalized();
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetPrayerReminderMinutes(minutes) => {
            state.settings.prayer.reminder_minutes = minutes;
            state.settings.prayer = state.settings.prayer.normalized();
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SetMediaVisualMode(mode) => {
            state.settings.music_visual_mode = mode;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::ResetAvatar => {
            let avatar = AvatarSettings::default();
            state.settings.blob_character = avatar.character;
            state.settings.blob_palette = avatar.palette;
            state.settings.blob_expression = avatar.expression;
            state.settings.blob_expression_custom = avatar.expression_custom;
            state.settings.avatar_random = avatar.random;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::ResetSettings => {
            let avatar = AvatarSettings::default();
            state.settings.blob_character = avatar.character;
            state.settings.blob_palette = avatar.palette;
            state.settings.blob_expression = avatar.expression;
            state.settings.blob_expression_custom = avatar.expression_custom;
            state.settings.avatar_random = avatar.random;
            state.settings.behavior = PetBehaviorSettings::default();
            state.settings.shadow = ShadowSettings::default();
            state.settings.prayer = PrayerSettings::default();
            state.settings.music_visual_mode = crate::models::MusicVisualMode::ArtworkPrimary;
            effects.push(Effect::PersistSettings);
        }

        AppEvent::SelectMediaSource(id) => {
            for source in &mut state.domain.media_sources {
                source.is_active = source.id == id;
            }
            effects.push(Effect::SelectMediaSource(id));
        }

        AppEvent::SpectrumUpdated(bars) => {
            state.domain.spectrum.bars = bars;
        }

        AppEvent::WindowOverlapChanged(overlapping) => {
            if !overlapping {
                state.runtime.overlap_dismiss_generation += 1;
            }
            state.view.engine.handle_overlap(overlapping);
            state.update_active_module();
        }

        AppEvent::UserHoverChanged(hovered) => {
            if !hovered
                && !state.view.engine.overlap_dismiss_active
                && state.view.engine.current_view == IslandView::Wide
                && state.view.engine.is_overlapping
            {
                state.runtime.overlap_reveal_generation += 1;
                state.runtime.overlap_dismiss_generation += 1;
                let generation = state.runtime.overlap_dismiss_generation;
                state.view.engine.begin_hover_overlap_dismiss();
                state.update_active_module();
                effects.push(Effect::ScheduleTimer {
                    id: TimerId::OverlapDismissHide,
                    duration_ms: 250,
                    generation,
                });
            } else {
                let was_hidden = !state.view.is_visible();
                let was_dismissing = state.view.engine.overlap_dismiss_active;
                let had_reveal_hold = state.view.engine.overlap_reveal_hold;

                state.view.engine.handle_hover(hovered);
                state.update_active_module();

                if was_dismissing && !state.view.engine.overlap_dismiss_active {
                    state.runtime.overlap_dismiss_generation += 1;
                }

                if !hovered {
                    state.runtime.overlap_reveal_generation += 1;
                } else if was_hidden && state.view.engine.overlap_reveal_hold && !had_reveal_hold {
                    state.runtime.overlap_reveal_generation += 1;
                    effects.push(Effect::ScheduleTimer {
                        id: TimerId::OverlapReveal,
                        duration_ms: 300,
                        generation: state.runtime.overlap_reveal_generation,
                    });
                }
            }
        }

        AppEvent::TimerFired {
            id: TimerId::OverlapReveal,
            generation,
        } => {
            if generation == state.runtime.overlap_reveal_generation {
                state.view.engine.finish_overlap_reveal();
                state.update_active_module();
            }
        }

        AppEvent::TimerFired {
            id: TimerId::OverlapDismissCompact,
            generation,
        } => {
            if generation == state.runtime.overlap_dismiss_generation
                && state.view.engine.overlap_dismiss_active
                && state.view.engine.is_overlapping
            {
                state.view.engine.advance_overlap_dismiss();
                state.update_active_module();
                effects.push(Effect::ScheduleTimer {
                    id: TimerId::OverlapDismissHide,
                    duration_ms: 250,
                    generation,
                });
            }
        }

        AppEvent::TimerFired {
            id: TimerId::OverlapDismissHide,
            generation,
        } => {
            if generation == state.runtime.overlap_dismiss_generation
                && state.view.engine.overlap_dismiss_active
                && state.view.engine.is_overlapping
            {
                state.view.engine.finish_overlap_dismiss();
                state.update_active_module();
            }
        }

        AppEvent::TimerFired {
            id: TimerId::CollapseCompact,
            generation,
        } => {
            if generation == state.runtime.overlap_dismiss_generation
                && state.view.engine.overlap_dismiss_active
                && !state.view.engine.is_overlapping
            {
                state.view.engine.advance_overlap_dismiss();
                state.view.engine.finish_overlap_dismiss();
                state.update_active_module();
            }
        }

        AppEvent::ViewSwitchRequested(target) => {
            state.runtime.overlap_reveal_generation += 1;
            state.runtime.overlap_dismiss_generation += 1;
            state.view.engine.cancel_overlap_sequences();
            state.view.engine.set_view(target);
            state.update_active_module();
        }

        AppEvent::ModuleSwitchRequested(module) => {
            let requested = module.into();
            let supported = state
                .view
                .engine
                .registry()
                .get(requested)
                .is_some_and(|module| {
                    module
                        .descriptor()
                        .tiers
                        .supports(state.view.engine.current_view)
                });
            if supported {
                state.update_active_module();
            }
        }

        AppEvent::TimerFired {
            id: TimerId::Welcome,
            generation,
        } => {
            if generation != state.presentation.welcome_generation {
                return effects;
            }
            state.presentation.is_welcome = false;
            // Settle into CompactView if not hovered
            if !state.view.engine.is_hovered {
                state.view.engine.set_view(IslandView::Compact);
            }
            state.update_active_module();
        }

        AppEvent::ScreenUnlocked | AppEvent::TriggerWelcome => {
            state.presentation.is_welcome = true;
            state.presentation.pet_is_happy = true;
            state.presentation.pet_happy_generation += 1;
            let happy_gen = state.presentation.pet_happy_generation;
            state.presentation.welcome_generation += 1;
            let gen_id = state.presentation.welcome_generation;
            state.runtime.overlap_reveal_generation += 1;
            state.runtime.overlap_dismiss_generation += 1;
            state.view.engine.cancel_overlap_sequences();
            state.view.engine.set_view(IslandView::Wide);
            state.update_active_module();
            effects.push(Effect::ScheduleTimer {
                id: TimerId::PetHappy,
                duration_ms: 1500,
                generation: happy_gen,
            });
            effects.push(Effect::ScheduleTimer {
                id: TimerId::Welcome,
                duration_ms: 3500,
                generation: gen_id,
            });
        }

        AppEvent::IslandClicked => {
            if !state.view.is_visible() && state.view.engine.is_overlapping {
                state.runtime.overlap_reveal_generation += 1;
                state.runtime.overlap_dismiss_generation += 1;
                state.view.engine.reveal_overlap_compact();
                state.update_active_module();
            } else if state.view.engine.current_view == IslandView::Expand {
                if state.view.engine.is_overlapping {
                    begin_overlap_dismissal(state, &mut effects);
                } else {
                    begin_standard_collapse(state, &mut effects);
                }
            } else {
                state.runtime.overlap_reveal_generation += 1;
                state.runtime.overlap_dismiss_generation += 1;
                state.view.engine.cancel_overlap_sequences();
                state.view.engine.handle_click();
                state.update_active_module();
            }
        }

        AppEvent::CollapseClicked => {
            if state.view.engine.current_view == IslandView::Expand {
                if state.view.engine.is_overlapping {
                    begin_overlap_dismissal(state, &mut effects);
                } else {
                    begin_standard_collapse(state, &mut effects);
                }
            } else {
                state.runtime.overlap_reveal_generation += 1;
                state.runtime.overlap_dismiss_generation += 1;
                state.view.engine.cancel_overlap_sequences();
                state.view.engine.handle_collapse();
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
            state.domain.is_dnd = is_dnd;
        }

        AppEvent::AudioMuteUpdated(is_muted) => {
            state.domain.is_muted = is_muted;
        }

        AppEvent::MuteToggleClicked => {
            state.domain.is_muted = !state.domain.is_muted;
            effects.push(Effect::ToggleMute);
        }

        AppEvent::DndToggleClicked => {
            state.domain.is_dnd = !state.domain.is_dnd;
            effects.push(Effect::ToggleDnd);
        }
    }

    effects
}

fn next_avatar_seed(state: &mut AppState) -> u64 {
    let mut x = state.settings.avatar_seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    state.settings.avatar_seed = x;
    x
}

fn randomize_avatar(state: &mut AppState) {
    state.settings.blob_character = (next_avatar_seed(state) % 8) as u32;
    state.settings.blob_palette = (next_avatar_seed(state) % 14) as u32;
    state.settings.blob_expression = (next_avatar_seed(state) % 17) as u32;
    state.settings.blob_expression_custom = true;
}
