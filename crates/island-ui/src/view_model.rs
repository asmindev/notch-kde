use crate::{ContentRouter, IslandContext, MainWindow, SettingsWindow};
use chrono::{Datelike, NaiveDate, Utc};
use island_core::models::ActiveModule;
use island_core::prayer::{
    moment_for, sky_for, times_for, Prayer, PrayerPhase, PrayerTimes, SkyState, CITIES,
};
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
            ActiveModule::Prayer => 3,
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
        sync_shadow_context(&ctx, state);
        sync_prayer_context(&ctx, state);
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
        sync_shadow_context(&ctx, state);
        sync_prayer_context(&ctx, state);
        ctx.set_music_visual_mode(state.music_visual_mode as i32);
        ctx.set_pet_is_happy(state.pet_is_happy);
        ctx.set_pet_is_dizzy(state.pet_is_dizzy);
        ctx.set_pet_is_sleepy(state.pet_is_sleepy && state.behavior.sleepy_on_low_battery);
        ctx.set_is_playing(false);
        ctx.set_app_version(env!("CARGO_PKG_VERSION").into());
    }
}

fn sync_shadow_context(ctx: &IslandContext, state: &AppState) {
    let shadow = state.shadow.normalized();
    let intensity = shadow.intensity as f32 / 100.0;
    let alpha = if shadow.enabled { intensity * 0.8 } else { 0.0 };
    ctx.set_shadow_enabled(shadow.enabled);
    ctx.set_shadow_intensity(shadow.intensity as i32);
    ctx.set_shadow_radius(shadow.radius as i32);
    ctx.set_shadow_offset_x(shadow.offset_x);
    ctx.set_shadow_offset_y(shadow.offset_y);
    ctx.set_shadow_color(slint::Color::from_argb_f32(alpha, 0.0, 0.0, 0.0));
}

fn sync_prayer_context(ctx: &IslandContext, state: &AppState) {
    ctx.set_prayer_enabled(state.prayer.enabled);
    ctx.set_prayer_panel_open(state.prayer_panel_open);
    ctx.set_prayer_city_index(state.prayer.city as i32);
    ctx.set_prayer_reminder_minutes(state.prayer.reminder_minutes as i32);

    if let Some(alert) = &state.prayer_alert {
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

fn sync_prayer_schedule(ctx: &IslandContext, state: &AppState) {
    let city = state.prayer.city();
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
        format!(
            "{} · {} · {}",
            city.name,
            city.region,
            format_date_id(date)
        )
        .into(),
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
fn push_sky(ctx: &IslandContext, city: &island_core::prayer::City, now: chrono::DateTime<chrono::FixedOffset>) {
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
    (
        rgb(sky.top),
        rgb(sky.horizon),
        rgb(sky.glow),
    )
}

fn rgb(c: [f32; 3]) -> slint::Color {
    slint::Color::from_rgb_f32(c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0))
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
    use island_core::prayer::{moment_for, times_for, CITIES};

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

        let subuh = markers.iter().find(|marker| marker.name == "Subuh").expect("Subuh marker");
        let isya = markers.iter().find(|marker| marker.name == "Isya").expect("Isya marker");
        assert!(subuh.is_next);
        assert!(!isya.is_next);
    }
}
