use crate::module_ui_registry::ModuleUiRegistry;
use crate::{ContentRouter, IslandContext, MainWindow, SettingsWindow};
use chrono::{Datelike, NaiveDate, Utc};
use island_core::models::IslandView;
use island_core::prayer::{
    CITIES, Prayer, PrayerPhase, PrayerTimes, SkyState, moment_for, sky_for, times_for,
};
use island_core::render_policy::RenderPolicy;
use island_core::snapshot::ViewSnapshot;
use slint::ComponentHandle;

pub struct ViewModel;

impl ViewModel {
    pub fn sync_state_to_ui(window: &MainWindow, state: &ViewSnapshot, policy: RenderPolicy) {
        let ctx = window.global::<IslandContext>();

        ctx.set_render_main_visible(policy.main_visible);
        ctx.set_render_pet(policy.pet_3d);
        ctx.set_render_spectrum(policy.spectrum);
        ctx.set_render_animations(policy.animations);
        ctx.set_active_mode(ContentRouter::mode_to_index(state.navigation.current_view));
        let module_ui = ModuleUiRegistry::default();
        ctx.set_compact_module(
            module_ui
                .slot_for(state.navigation.active_module, IslandView::Compact)
                .index(),
        );
        ctx.set_wide_module(
            module_ui
                .slot_for(state.navigation.active_module, IslandView::Wide)
                .index(),
        );
        ctx.set_expand_module(
            module_ui
                .slot_for(state.navigation.active_module, IslandView::Expand)
                .index(),
        );
        ctx.set_alert_kind(state.domain.alert_kind as i32);
        ctx.set_is_visible_state(state.navigation.is_visible);

        ctx.set_time_str(state.domain.time.time_str.clone().into());
        ctx.set_date_str(state.domain.time.date_str.clone().into());

        ctx.set_battery_percentage(state.domain.battery.percentage as i32);
        ctx.set_is_charging(state.domain.battery.is_charging);

        ctx.set_song_title(state.domain.media.title.clone().into());
        ctx.set_song_artist(state.domain.media.artist.clone().into());
        ctx.set_is_playing(state.domain.media.is_playing);
        ctx.set_has_audio(
            state.domain.media.is_playing || state.domain.spectrum.iter().any(|&b| b > 0.05),
        );
        ctx.set_music_visual_mode(state.settings.music_visual_mode as i32);
        ctx.set_mochi_mood(state.presentation.mochi_mood as i32);
        ctx.set_blob_character(state.settings.blob_character as i32);
        ctx.set_blob_palette(state.settings.blob_palette as i32);
        ctx.set_blob_expression(state.settings.blob_expression as i32);
        ctx.set_avatar_random(state.settings.avatar_random);
        ctx.set_settings_open(state.presentation.settings_open);
        ctx.set_settings_tab(state.presentation.settings_tab as i32);
        ctx.set_cursor_gaze_enabled(state.settings.behavior.cursor_gaze);
        ctx.set_idle_acting_enabled(state.settings.behavior.idle_acting);
        ctx.set_click_stunts_enabled(state.settings.behavior.click_stunts);
        ctx.set_notification_reactions_enabled(state.settings.behavior.notification_reactions);
        ctx.set_music_reactions_enabled(state.settings.behavior.music_reactions);
        ctx.set_sleepy_on_low_battery(state.settings.behavior.sleepy_on_low_battery);
        sync_shadow_context(&ctx, state);
        sync_prayer_context(&ctx, state);
        ctx.set_pet_is_happy(state.presentation.pet_is_happy);
        ctx.set_pet_is_dizzy(state.presentation.pet_is_dizzy);
        ctx.set_pet_is_sleepy(
            state.presentation.pet_is_sleepy && state.settings.behavior.sleepy_on_low_battery,
        );
        ctx.set_is_welcome(state.presentation.is_welcome);
        ctx.set_welcome_title(state.presentation.welcome_title.clone().into());
        ctx.set_welcome_subtitle(state.presentation.welcome_subtitle.clone().into());
        ctx.set_app_version(env!("CARGO_PKG_VERSION").into());

        // Resolve Album Art image
        if !state.domain.media.art_url.is_empty() {
            let path = std::path::Path::new(&state.domain.media.art_url);
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

        ctx.set_is_muted(state.domain.is_muted);
        ctx.set_is_dnd(state.domain.is_dnd);

        // Media sources selection
        ctx.set_show_source_selector(state.presentation.show_source_selector);
        let items: Vec<crate::PlayerSourceItem> = state
            .domain
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

        if policy.spectrum {
            Self::sync_spectrum_to_ui(window, &state.domain.spectrum);
        }
    }

    pub fn sync_spectrum_to_ui(window: &MainWindow, bars: &[f32; 8]) {
        let ctx = window.global::<IslandContext>();
        ctx.set_b0(bars[0]);
        ctx.set_b1(bars[1]);
        ctx.set_b2(bars[2]);
        ctx.set_b3(bars[3]);
        ctx.set_b4(bars[4]);
        ctx.set_b5(bars[5]);
        ctx.set_b6(bars[6]);
        ctx.set_b7(bars[7]);
    }

    pub fn clear_spectrum_to_ui(window: &MainWindow) {
        Self::sync_spectrum_to_ui(window, &[0.0; 8]);
    }

    pub fn sync_settings_to_ui(settings: &SettingsWindow, state: &ViewSnapshot) {
        let ctx = settings.global::<IslandContext>();

        ctx.set_settings_open(state.presentation.settings_open);
        ctx.set_settings_tab(state.presentation.settings_tab as i32);
        ctx.set_blob_character(state.settings.blob_character as i32);
        ctx.set_blob_palette(state.settings.blob_palette as i32);
        ctx.set_blob_expression(state.settings.blob_expression as i32);
        ctx.set_avatar_random(state.settings.avatar_random);
        ctx.set_cursor_gaze_enabled(state.settings.behavior.cursor_gaze);
        ctx.set_idle_acting_enabled(state.settings.behavior.idle_acting);
        ctx.set_click_stunts_enabled(state.settings.behavior.click_stunts);
        ctx.set_notification_reactions_enabled(state.settings.behavior.notification_reactions);
        ctx.set_music_reactions_enabled(state.settings.behavior.music_reactions);
        ctx.set_sleepy_on_low_battery(state.settings.behavior.sleepy_on_low_battery);
        sync_shadow_context(&ctx, state);
        sync_prayer_context(&ctx, state);
        ctx.set_music_visual_mode(state.settings.music_visual_mode as i32);
        ctx.set_pet_is_happy(state.presentation.pet_is_happy);
        ctx.set_pet_is_dizzy(state.presentation.pet_is_dizzy);
        ctx.set_pet_is_sleepy(
            state.presentation.pet_is_sleepy && state.settings.behavior.sleepy_on_low_battery,
        );
        ctx.set_is_playing(false);
        ctx.set_app_version(env!("CARGO_PKG_VERSION").into());
    }
}

fn sync_shadow_context(ctx: &IslandContext, state: &ViewSnapshot) {
    let shadow = state.settings.shadow.normalized();
    let intensity = shadow.intensity as f32 / 100.0;
    let alpha = if shadow.enabled { intensity * 0.8 } else { 0.0 };
    ctx.set_shadow_enabled(shadow.enabled);
    ctx.set_shadow_intensity(shadow.intensity as i32);
    ctx.set_shadow_radius(shadow.radius as i32);
    ctx.set_shadow_offset_x(shadow.offset_x);
    ctx.set_shadow_offset_y(shadow.offset_y);
    ctx.set_shadow_color(slint::Color::from_argb_f32(alpha, 0.0, 0.0, 0.0));
}

fn sync_prayer_context(ctx: &IslandContext, state: &ViewSnapshot) {
    ctx.set_prayer_enabled(state.settings.prayer.enabled);
    ctx.set_prayer_panel_open(state.presentation.prayer_panel_open);
    ctx.set_prayer_city_index(state.settings.prayer.city as i32);
    ctx.set_prayer_reminder_minutes(state.settings.prayer.reminder_minutes as i32);

    if let Some(alert) = &state.domain.prayer_alert {
        ctx.set_prayer_title(alert.title.clone().into());
        ctx.set_prayer_time(alert.time_label.clone().into());
        ctx.set_prayer_city_label(alert.city_label.clone().into());
    } else {
        ctx.set_prayer_title("".into());
        ctx.set_prayer_time("".into());
        ctx.set_prayer_city_label("".into());
    }

    let labels: Vec<slint::SharedString> = CITIES
        .iter()
        .map(|city| format!("{} · {}", city.name, city.region).into())
        .collect();
    ctx.set_prayer_city_labels(slint::ModelRc::new(slint::VecModel::from(labels)));

    sync_prayer_schedule(ctx, state);
}

fn sync_prayer_schedule(ctx: &IslandContext, state: &ViewSnapshot) {
    let city = state.settings.prayer.city();
    let now = Utc::now().with_timezone(&city.offset());
    let date = now.date_naive();
    ctx.set_prayer_now_time(now.format("%H:%M").to_string().into());

    let Some(times) = times_for(city, date) else {
        let empty: Vec<crate::PrayerTimeItem> = Vec::new();
        ctx.set_prayer_schedule(slint::ModelRc::new(slint::VecModel::from(empty)));
        ctx.set_prayer_schedule_subtitle("Jadwal tidak tersedia untuk tanggal ini.".into());
        ctx.set_prayer_next_name("".into());
        ctx.set_prayer_next_time("".into());
        ctx.set_prayer_countdown("".into());
        reset_prayer_orbit(ctx);
        return;
    };

    let next_index = times.times.iter().position(|time| time > &now);
    let schedule: Vec<crate::PrayerTimeItem> = times
        .iter()
        .enumerate()
        .map(|(index, (prayer, time))| crate::PrayerTimeItem {
            name: prayer.name_id().into(),
            time: time.format("%H:%M").to_string().into(),
            is_next: Some(index) == next_index,
        })
        .collect();
    ctx.set_prayer_schedule(slint::ModelRc::new(slint::VecModel::from(schedule)));
    ctx.set_prayer_schedule_subtitle(
        format!("{} · {} · {}", city.name, city.region, format_date_id(date)).into(),
    );

    let Some(moment) = moment_for(city, now) else {
        ctx.set_prayer_next_name("".into());
        ctx.set_prayer_next_time("".into());
        ctx.set_prayer_countdown("".into());
        reset_prayer_orbit(ctx);
        return;
    };

    ctx.set_prayer_next_name(moment.next_prayer.name_id().into());
    ctx.set_prayer_next_time(moment.next_at.format("%H:%M").to_string().into());
    let countdown = (moment.next_at - now).num_minutes().max(0);
    ctx.set_prayer_countdown(format_countdown(countdown).into());
    ctx.set_prayer_orbit_progress(moment.orbit_progress);
    ctx.set_prayer_orbit_night(moment.phase == PrayerPhase::Night);
    let markers = line_markers(&times, moment.next_prayer);
    ctx.set_prayer_orbit_markers(slint::ModelRc::new(slint::VecModel::from(markers)));

    push_sky(ctx, city, now);
}

fn reset_prayer_orbit(ctx: &IslandContext) {
    ctx.set_prayer_orbit_progress(0.0);
    ctx.set_prayer_orbit_night(false);
    let markers: Vec<crate::PrayerOrbitMarker> = Vec::new();
    ctx.set_prayer_orbit_markers(slint::ModelRc::new(slint::VecModel::from(markers)));
    let night_sky = sky_from(SkyState {
        top: [0.02, 0.03, 0.08],
        horizon: [0.03, 0.05, 0.12],
        glow: [0.06, 0.08, 0.14],
        star_opacity: 0.0,
    });
    ctx.set_prayer_sky_top(night_sky.0);
    ctx.set_prayer_sky_horizon(night_sky.1);
    ctx.set_prayer_sky_glow(night_sky.2);
    ctx.set_prayer_star_opacity(0.0);
}

/// Compute the real sky for the active city and push the derived colors.
fn push_sky(
    ctx: &IslandContext,
    city: &island_core::prayer::City,
    now: chrono::DateTime<chrono::FixedOffset>,
) {
    // Fall back to a neutral deep night if the city/date is out of range.
    let sky = sky_for(city, now).unwrap_or(SkyState {
        top: [0.02, 0.03, 0.08],
        horizon: [0.03, 0.05, 0.12],
        glow: [0.06, 0.08, 0.14],
        star_opacity: 0.0,
    });
    let (top, horizon, glow) = sky_from(sky);
    ctx.set_prayer_sky_top(top);
    ctx.set_prayer_sky_horizon(horizon);
    ctx.set_prayer_sky_glow(glow);
    ctx.set_prayer_star_opacity(sky.star_opacity);
}

/// Convert an `SkyState`'s sRGB triples into three Slint colors.
fn sky_from(sky: SkyState) -> (slint::Color, slint::Color, slint::Color) {
    (rgb(sky.top), rgb(sky.horizon), rgb(sky.glow))
}

fn rgb(c: [f32; 3]) -> slint::Color {
    slint::Color::from_rgb_f32(
        c[0].clamp(0.0, 1.0),
        c[1].clamp(0.0, 1.0),
        c[2].clamp(0.0, 1.0),
    )
}

fn line_markers(times: &PrayerTimes, next: Prayer) -> Vec<crate::PrayerOrbitMarker> {
    times
        .iter()
        .enumerate()
        .map(|(index, (prayer, time))| crate::PrayerOrbitMarker {
            name: prayer.name_id().into(),
            time: time.format("%H:%M").to_string().into(),
            x: 0.02 + index as f32 * 0.24,
            is_next: prayer == next,
        })
        .collect()
}

fn format_countdown(minutes: i64) -> String {
    match minutes {
        ..=0 => "Sekarang".to_string(),
        1..=59 => format!("{minutes} mnt"),
        _ if minutes % 60 == 0 => format!("{} jam", minutes / 60),
        _ => format!("{}j {}m", minutes / 60, minutes % 60),
    }
}

fn format_date_id(date: NaiveDate) -> String {
    const WEEKDAYS: [&str; 7] = [
        "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu", "Minggu",
    ];
    const MONTHS: [&str; 12] = [
        "Januari",
        "Februari",
        "Maret",
        "April",
        "Mei",
        "Juni",
        "Juli",
        "Agustus",
        "September",
        "Oktober",
        "November",
        "Desember",
    ];
    format!(
        "{}, {} {} {}",
        WEEKDAYS[date.weekday().num_days_from_monday() as usize],
        date.day(),
        MONTHS[date.month0() as usize],
        date.year()
    )
}

#[cfg(test)]
mod tests {
    use super::{format_date_id, line_markers};
    use chrono::{NaiveDate, TimeZone};
    use island_core::prayer::{CITIES, moment_for, times_for};

    #[test]
    fn formats_indonesian_date() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        assert_eq!(format_date_id(date), "Kamis, 29 Oktober 2026");
    }

    #[test]
    fn line_markers_are_evenly_spaced_and_highlight_next() {
        let city = &CITIES[0];
        let date = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
        let now = city
            .offset()
            .with_ymd_and_hms(2026, 10, 28, 23, 0, 0)
            .unwrap();
        let moment = moment_for(city, now).expect("valid moment");
        let times = times_for(city, date).expect("valid times");

        let markers = line_markers(&times, moment.next_prayer);
        assert_eq!(markers.len(), 5);
        for (index, marker) in markers.iter().enumerate() {
            let expected = 0.02 + index as f32 * 0.24;
            assert!((marker.x - expected).abs() < f32::EPSILON);
        }

        let subuh = markers
            .iter()
            .find(|marker| marker.name == "Subuh")
            .expect("Subuh marker");
        let isya = markers
            .iter()
            .find(|marker| marker.name == "Isya")
            .expect("Isya marker");
        assert!(subuh.is_next);
        assert!(!isya.is_next);
    }
}
