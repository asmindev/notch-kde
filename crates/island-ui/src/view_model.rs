use crate::{ContentRouter, IslandContext, MainWindow, SettingsWindow};
use island_core::models::ActiveModule;
use island_core::state::AppState;
use slint::ComponentHandle;

pub struct ViewModel;

impl ViewModel {
    pub fn sync_state_to_ui(window: &MainWindow, state: &AppState) {
        let ctx = window.global::<IslandContext>();

        ctx.set_active_mode(ContentRouter::mode_to_index(state.mode));
        ctx.set_active_module(match state.active_module {
            ActiveModule::System => 0,
            ActiveModule::Media => 1,
            ActiveModule::Alert => 2,
        });
        ctx.set_alert_kind(state.alert_kind as i32);
        ctx.set_is_visible_state(state.is_visible);

        ctx.set_time_str(state.time.time_str.clone().into());
        ctx.set_date_str(state.time.date_str.clone().into());

        ctx.set_battery_percentage(state.battery.percentage as i32);
        ctx.set_is_charging(state.battery.is_charging);

        ctx.set_song_title(state.media.title.clone().into());
        ctx.set_song_artist(state.media.artist.clone().into());
        ctx.set_is_playing(state.media.is_playing);
        ctx.set_has_audio(state.media.is_playing || state.spectrum.bars.iter().any(|&b| b > 0.05));
        ctx.set_music_visual_mode(state.music_visual_mode as i32);
        ctx.set_mochi_mood(state.mochi_mood as i32);
        ctx.set_blob_character(state.blob_character as i32);
        ctx.set_blob_palette(state.blob_palette as i32);
        ctx.set_blob_expression(state.blob_expression as i32);
        ctx.set_avatar_random(state.avatar_random);
        ctx.set_settings_open(state.settings_open);
        ctx.set_settings_tab(state.settings_tab as i32);
        ctx.set_cursor_gaze_enabled(state.behavior.cursor_gaze);
        ctx.set_idle_acting_enabled(state.behavior.idle_acting);
        ctx.set_click_stunts_enabled(state.behavior.click_stunts);
        ctx.set_notification_reactions_enabled(state.behavior.notification_reactions);
        ctx.set_music_reactions_enabled(state.behavior.music_reactions);
        ctx.set_sleepy_on_low_battery(state.behavior.sleepy_on_low_battery);
        ctx.set_pet_is_happy(state.pet_is_happy);
        ctx.set_pet_is_dizzy(state.pet_is_dizzy);
        ctx.set_pet_is_sleepy(state.pet_is_sleepy && state.behavior.sleepy_on_low_battery);
        ctx.set_is_welcome(state.is_welcome);
        ctx.set_welcome_title(state.welcome_title.clone().into());
        ctx.set_welcome_subtitle(state.welcome_subtitle.clone().into());
        ctx.set_app_version(env!("CARGO_PKG_VERSION").into());

        // Resolve Album Art image
        if !state.media.art_url.is_empty() {
            let path = std::path::Path::new(&state.media.art_url);
            if path.exists() {
                if let Ok(img) = slint::Image::load_from_path(path) {
                    ctx.set_art_image(img);
                    ctx.set_has_art(true);
                } else {
                    ctx.set_has_art(false);
                }
            } else {
                ctx.set_has_art(false);
            }
        } else {
            ctx.set_has_art(false);
        }

        ctx.set_is_muted(state.is_muted);
        ctx.set_is_dnd(state.is_dnd);

        // Media sources selection
        ctx.set_show_source_selector(state.show_source_selector);
        let items: Vec<crate::PlayerSourceItem> = state
            .media_sources
            .iter()
            .map(|s| crate::PlayerSourceItem {
                id: s.id.clone().into(),
                name: s.name.clone().into(),
                is_active: s.is_active,
                is_playing: s.is_playing,
            })
            .collect();
        ctx.set_player_sources(slint::ModelRc::new(slint::VecModel::from(items)));

        // Spectrum bars
        ctx.set_b0(state.spectrum.bars[0]);
        ctx.set_b1(state.spectrum.bars[1]);
        ctx.set_b2(state.spectrum.bars[2]);
        ctx.set_b3(state.spectrum.bars[3]);
        ctx.set_b4(state.spectrum.bars[4]);
        ctx.set_b5(state.spectrum.bars[5]);
        ctx.set_b6(state.spectrum.bars[6]);
        ctx.set_b7(state.spectrum.bars[7]);
    }

    pub fn sync_settings_to_ui(settings: &SettingsWindow, state: &AppState) {
        let ctx = settings.global::<IslandContext>();

        ctx.set_settings_open(state.settings_open);
        ctx.set_settings_tab(state.settings_tab as i32);
        ctx.set_blob_character(state.blob_character as i32);
        ctx.set_blob_palette(state.blob_palette as i32);
        ctx.set_blob_expression(state.blob_expression as i32);
        ctx.set_avatar_random(state.avatar_random);
        ctx.set_cursor_gaze_enabled(state.behavior.cursor_gaze);
        ctx.set_idle_acting_enabled(state.behavior.idle_acting);
        ctx.set_click_stunts_enabled(state.behavior.click_stunts);
        ctx.set_notification_reactions_enabled(state.behavior.notification_reactions);
        ctx.set_music_reactions_enabled(state.behavior.music_reactions);
        ctx.set_sleepy_on_low_battery(state.behavior.sleepy_on_low_battery);
        ctx.set_music_visual_mode(state.music_visual_mode as i32);
        ctx.set_pet_is_happy(state.pet_is_happy);
        ctx.set_pet_is_dizzy(state.pet_is_dizzy);
        ctx.set_pet_is_sleepy(state.pet_is_sleepy && state.behavior.sleepy_on_low_battery);
        ctx.set_is_playing(false);
        ctx.set_app_version(env!("CARGO_PKG_VERSION").into());
    }
}
