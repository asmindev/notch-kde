use crate::module_ui_registry::ModuleUiRegistry;
use crate::{ContentRouter, IslandContext, MainWindow, SettingsWindow};
use chrono::{Datelike, NaiveDate, Timelike, Utc};
use island_core::models::{IslandView, MediaSource};
use island_core::prayer::{
    CITIES, Prayer, PrayerPhase, PrayerTimes, SkyState, moment_for, sky_for, times_for,
};
use island_core::render_policy::RenderPolicy;
use island_core::snapshot::ViewSnapshot;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::SystemTime;

const ART_CACHE_CAPACITY: usize = 4;

thread_local! {
    static MAIN_SYNC_CACHE: RefCell<UiSyncCache> = RefCell::new(UiSyncCache::default());
    static SETTINGS_SYNC_CACHE: RefCell<UiSyncCache> = RefCell::new(UiSyncCache::default());
}

pub struct ViewModel;

impl ViewModel {
    pub fn sync_state_to_ui(window: &MainWindow, state: &ViewSnapshot, policy: RenderPolicy) {
        MAIN_SYNC_CACHE.with(|cache| cache.borrow_mut().sync_main(window, state, policy));
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
        SETTINGS_SYNC_CACHE.with(|cache| cache.borrow_mut().sync_settings(settings, state));
    }

    fn sync_render_flags(ctx: &IslandContext, policy: RenderPolicy) {
        ctx.set_render_main_visible(policy.main_visible);
        ctx.set_render_pet(policy.pet_3d);
        ctx.set_render_spectrum(policy.spectrum);
        ctx.set_render_animations(policy.animations);
    }

    fn sync_navigation(ctx: &IslandContext, state: &ViewSnapshot) {
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
    }

    fn sync_time(ctx: &IslandContext, state: &ViewSnapshot) {
        ctx.set_time_str(state.domain.time.time_str.clone().into());
        ctx.set_date_str(state.domain.time.date_str.clone().into());
    }

    fn sync_battery(ctx: &IslandContext, state: &ViewSnapshot) {
        ctx.set_battery_percentage(state.domain.battery.percentage as i32);
        ctx.set_is_charging(state.domain.battery.is_charging);
        ctx.set_is_muted(state.domain.is_muted);
        ctx.set_is_dnd(state.domain.is_dnd);
    }

    fn sync_media(ctx: &IslandContext, state: &ViewSnapshot) {
        ctx.set_song_title(state.domain.media.title.clone().into());
        ctx.set_song_artist(state.domain.media.artist.clone().into());
        ctx.set_is_playing(state.domain.media.is_playing);
        ctx.set_has_audio(
            state.domain.media.is_playing || state.domain.spectrum.iter().any(|&b| b > 0.05),
        );
        ctx.set_show_source_selector(state.presentation.show_source_selector);
    }

    fn sync_settings_group(ctx: &IslandContext, state: &ViewSnapshot) {
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
        ctx.set_music_visual_mode(state.settings.music_visual_mode as i32);
        ctx.set_pet_renderer(if state.settings.pet_renderer.is_3d() {
            0
        } else {
            1
        });
        ctx.set_compact_mode_style(state.settings.compact_mode_style as i32);
        sync_shadow_context(ctx, state);
    }

    fn sync_pet_flags(ctx: &IslandContext, state: &ViewSnapshot) {
        ctx.set_pet_is_happy(state.presentation.pet_is_happy);
        ctx.set_pet_is_dizzy(state.presentation.pet_is_dizzy);
        ctx.set_pet_is_sleepy(
            state.presentation.pet_is_sleepy && state.settings.behavior.sleepy_on_low_battery,
        );
    }

    fn sync_presentation(ctx: &IslandContext, state: &ViewSnapshot) {
        ctx.set_mochi_mood(state.presentation.mochi_mood as i32);
        Self::sync_pet_flags(ctx, state);
        ctx.set_is_welcome(state.presentation.is_welcome);
        ctx.set_welcome_title(state.presentation.welcome_title.clone().into());
        ctx.set_welcome_subtitle(state.presentation.welcome_subtitle.clone().into());
    }

    fn sync_prayer_shared(ctx: &IslandContext, state: &ViewSnapshot, cache: &mut PrayerCache) {
        ctx.set_prayer_enabled(state.settings.prayer.enabled);
        ctx.set_prayer_city_index(state.settings.prayer.city as i32);
        ctx.set_prayer_reminder_minutes(state.settings.prayer.reminder_minutes as i32);

        if cache.city_labels.is_none() {
            let labels: Vec<slint::SharedString> = CITIES
                .iter()
                .map(|city| format!("{} · {}", city.name, city.region).into())
                .collect();
            cache.city_labels = Some(slint::ModelRc::new(slint::VecModel::from(labels)));
        }
        if let Some(labels) = &cache.city_labels {
            ctx.set_prayer_city_labels(labels.clone());
        }
    }

    fn sync_prayer_context(ctx: &IslandContext, state: &ViewSnapshot, cache: &mut PrayerCache) {
        ctx.set_prayer_panel_open(state.presentation.prayer_panel_open);

        if let Some(alert) = &state.domain.prayer_alert {
            ctx.set_prayer_title(alert.title.clone().into());
            ctx.set_prayer_time(alert.time_label.clone().into());
            ctx.set_prayer_city_label(alert.city_label.clone().into());
        } else {
            ctx.set_prayer_title("".into());
            ctx.set_prayer_time("".into());
            ctx.set_prayer_city_label("".into());
        }

        Self::sync_prayer_shared(ctx, state, cache);
        sync_prayer_schedule(ctx, state, cache, PrayerScope::Full);
    }

    fn sync_prayer_settings(ctx: &IslandContext, state: &ViewSnapshot, cache: &mut PrayerCache) {
        Self::sync_prayer_shared(ctx, state, cache);
        sync_prayer_schedule(ctx, state, cache, PrayerScope::Settings);
    }
}

#[derive(Default)]
struct UiSyncCache {
    last: Option<ViewSnapshot>,
    last_policy: Option<RenderPolicy>,
    version_set: bool,
    art: ArtCache,
    sources: SourceCache,
    prayer: PrayerCache,
}

impl UiSyncCache {
    fn sync_main(&mut self, window: &MainWindow, state: &ViewSnapshot, policy: RenderPolicy) {
        let ctx = window.global::<IslandContext>();
        let policy_changed = self.last_policy != Some(policy);
        if policy_changed {
            ViewModel::sync_render_flags(&ctx, policy);
            self.last_policy = Some(policy);
        }
        if !self.version_set {
            ctx.set_app_version(env!("CARGO_PKG_VERSION").into());
            self.version_set = true;
        }

        let groups = self.groups(state);
        if groups.navigation {
            ViewModel::sync_navigation(&ctx, state);
        }
        if groups.time {
            ViewModel::sync_time(&ctx, state);
        }
        if groups.battery {
            ViewModel::sync_battery(&ctx, state);
        }
        if groups.media {
            self.sync_media(&ctx, state);
            if policy.spectrum {
                ViewModel::sync_spectrum_to_ui(window, &state.domain.spectrum);
            }
        } else if policy_changed && policy.spectrum {
            ViewModel::sync_spectrum_to_ui(window, &state.domain.spectrum);
        }
        if groups.prayer {
            ViewModel::sync_prayer_context(&ctx, state, &mut self.prayer);
        }
        if groups.settings {
            ViewModel::sync_settings_group(&ctx, state);
        }
        if groups.presentation {
            ViewModel::sync_presentation(&ctx, state);
        }

        self.last = Some(state.clone());
    }

    fn sync_settings(&mut self, settings: &SettingsWindow, state: &ViewSnapshot) {
        let ctx = settings.global::<IslandContext>();
        if !self.version_set {
            ctx.set_app_version(env!("CARGO_PKG_VERSION").into());
            self.version_set = true;
        }

        let groups = self.groups(state);
        if groups.settings {
            ViewModel::sync_settings_group(&ctx, state);
        }
        if groups.prayer {
            ViewModel::sync_prayer_settings(&ctx, state, &mut self.prayer);
        }
        if groups.presentation {
            ViewModel::sync_pet_flags(&ctx, state);
        }
        ctx.set_is_playing(false);

        self.last = Some(state.clone());
    }

    fn groups(&self, state: &ViewSnapshot) -> SyncGroups {
        self.last
            .as_ref()
            .map(|previous| SyncGroups::between(previous, state))
            .unwrap_or_else(SyncGroups::all)
    }

    fn sync_media(&mut self, ctx: &IslandContext, state: &ViewSnapshot) {
        ViewModel::sync_media(ctx, state);

        match self.art.resolve(&state.domain.media.art_url) {
            ArtUpdate::Unchanged => {}
            ArtUpdate::Update(Some(image)) => {
                ctx.set_art_image(image);
                ctx.set_has_art(true);
            }
            ArtUpdate::Update(None) => ctx.set_has_art(false),
        }

        if self.sources.changed(&state.domain.media_sources) {
            let items: Vec<crate::PlayerSourceItem> = state
                .domain
                .media_sources
                .iter()
                .map(|source| crate::PlayerSourceItem {
                    id: source.id.clone().into(),
                    name: source.name.clone().into(),
                    is_active: source.is_active,
                    is_playing: source.is_playing,
                })
                .collect();
            ctx.set_player_sources(slint::ModelRc::new(slint::VecModel::from(items)));
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SyncGroups {
    navigation: bool,
    time: bool,
    battery: bool,
    media: bool,
    prayer: bool,
    settings: bool,
    presentation: bool,
}

impl SyncGroups {
    fn all() -> Self {
        Self {
            navigation: true,
            time: true,
            battery: true,
            media: true,
            prayer: true,
            settings: true,
            presentation: true,
        }
    }

    fn between(previous: &ViewSnapshot, next: &ViewSnapshot) -> Self {
        let time = previous.domain.time != next.domain.time;
        Self {
            navigation: previous.navigation != next.navigation,
            time,
            battery: previous.domain.battery != next.domain.battery
                || previous.domain.is_muted != next.domain.is_muted
                || previous.domain.is_dnd != next.domain.is_dnd,
            media: previous.domain.media != next.domain.media
                || previous.domain.media_sources != next.domain.media_sources
                || previous.domain.spectrum != next.domain.spectrum
                || previous.presentation.show_source_selector
                    != next.presentation.show_source_selector,
            prayer: previous.domain.prayer_alert != next.domain.prayer_alert
                || previous.settings.prayer != next.settings.prayer
                || previous.presentation.prayer_panel_open != next.presentation.prayer_panel_open
                || time,
            settings: previous.settings != next.settings
                || previous.presentation.settings_open != next.presentation.settings_open
                || previous.presentation.settings_tab != next.presentation.settings_tab,
            presentation: previous.presentation.mochi_mood != next.presentation.mochi_mood
                || previous.presentation.pet_is_happy != next.presentation.pet_is_happy
                || previous.presentation.pet_is_dizzy != next.presentation.pet_is_dizzy
                || previous.presentation.pet_is_sleepy != next.presentation.pet_is_sleepy
                || previous.presentation.is_welcome != next.presentation.is_welcome
                || previous.presentation.welcome_title != next.presentation.welcome_title
                || previous.presentation.welcome_subtitle != next.presentation.welcome_subtitle
                || previous.settings.behavior != next.settings.behavior,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtKey {
    path: PathBuf,
    modified: Option<SystemTime>,
    length: Option<u64>,
}

enum ArtUpdate {
    Unchanged,
    Update(Option<slint::Image>),
}

#[derive(Default)]
struct ArtCache {
    entries: VecDeque<(ArtKey, slint::Image)>,
    current: Option<ArtKey>,
    resolved: bool,
}

impl ArtCache {
    fn resolve(&mut self, art_url: &str) -> ArtUpdate {
        let key = art_key(art_url);
        if self.resolved && self.current == key {
            return ArtUpdate::Unchanged;
        }
        self.resolved = true;
        self.current = key.clone();

        let Some(key) = key else {
            return ArtUpdate::Update(None);
        };

        if let Some(index) = self.entries.iter().position(|(cached, _)| cached == &key)
            && let Some(entry) = self.entries.remove(index)
        {
            let image = entry.1.clone();
            self.entries.push_front(entry);
            return ArtUpdate::Update(Some(image));
        }

        let image = slint::Image::load_from_path(&key.path).ok();
        if let Some(image) = &image {
            self.entries.push_front((key, image.clone()));
            if self.entries.len() > ART_CACHE_CAPACITY {
                self.entries.pop_back();
            }
        }
        ArtUpdate::Update(image)
    }
}

fn art_key(art_url: &str) -> Option<ArtKey> {
    if art_url.is_empty() {
        return None;
    }
    let path = PathBuf::from(art_url);
    let metadata = std::fs::metadata(&path).ok();
    Some(ArtKey {
        path,
        modified: metadata.as_ref().and_then(|meta| meta.modified().ok()),
        length: metadata.map(|meta| meta.len()),
    })
}

#[derive(Default)]
struct SourceCache {
    last: Option<Vec<MediaSource>>,
}

impl SourceCache {
    fn changed(&mut self, sources: &[MediaSource]) -> bool {
        if self.last.as_deref() == Some(sources) {
            return false;
        }
        self.last = Some(sources.to_vec());
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrayerScheduleKey {
    city: u32,
    date: NaiveDate,
    minute: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrayerScope {
    Full,
    Settings,
}

#[derive(Default)]
struct PrayerCache {
    schedule_key: Option<PrayerScheduleKey>,
    city_labels: Option<slint::ModelRc<slint::SharedString>>,
}

impl PrayerCache {
    fn needs_refresh(&mut self, key: PrayerScheduleKey) -> bool {
        if self.schedule_key.as_ref() == Some(&key) {
            return false;
        }
        self.schedule_key = Some(key);
        true
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

fn sync_prayer_schedule(
    ctx: &IslandContext,
    state: &ViewSnapshot,
    cache: &mut PrayerCache,
    scope: PrayerScope,
) {
    let city = state.settings.prayer.city();
    let now = Utc::now().with_timezone(&city.offset());
    let key = PrayerScheduleKey {
        city: state.settings.prayer.city % CITIES.len() as u32,
        date: now.date_naive(),
        minute: now.hour() * 60 + now.minute(),
    };
    if !cache.needs_refresh(key) {
        return;
    }

    if scope == PrayerScope::Full {
        ctx.set_prayer_now_time(now.format("%H:%M").to_string().into());
    }

    let date = now.date_naive();
    let Some(times) = times_for(city, date) else {
        let empty: Vec<crate::PrayerTimeItem> = Vec::new();
        ctx.set_prayer_schedule(slint::ModelRc::new(slint::VecModel::from(empty)));
        ctx.set_prayer_schedule_subtitle("Jadwal tidak tersedia untuk tanggal ini.".into());
        if scope == PrayerScope::Full {
            ctx.set_prayer_next_name("".into());
            ctx.set_prayer_next_time("".into());
            ctx.set_prayer_countdown("".into());
            reset_prayer_orbit(ctx);
        }
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

    if scope == PrayerScope::Settings {
        return;
    }

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
    use super::{
        ART_CACHE_CAPACITY, ArtCache, ArtUpdate, PrayerCache, PrayerScheduleKey, SourceCache,
        SyncGroups, art_key, format_date_id, line_markers,
    };
    use chrono::{NaiveDate, TimeZone};
    use island_core::AppState;
    use island_core::models::MediaSource;
    use island_core::prayer::{CITIES, moment_for, times_for};
    use island_core::snapshot::ViewSnapshot;

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

    #[test]
    fn spectrum_change_only_marks_media_group() {
        let previous = ViewSnapshot::from_state(&AppState::default());
        let mut next = previous.clone();
        next.domain.spectrum = [0.5; 8];

        let groups = SyncGroups::between(&previous, &next);

        assert!(groups.media);
        assert!(!groups.navigation);
        assert!(!groups.time);
        assert!(!groups.prayer);
        assert!(!groups.settings);
        assert!(!groups.presentation);
    }

    #[test]
    fn non_prayer_settings_do_not_refresh_prayer_group() {
        let previous = ViewSnapshot::from_state(&AppState::default());
        let mut next = previous.clone();
        next.settings.shadow.intensity = 70;

        let groups = SyncGroups::between(&previous, &next);

        assert!(groups.settings);
        assert!(!groups.presentation);
        assert!(!groups.prayer);
    }

    #[test]
    fn clock_tick_refreshes_prayer_countdown_group() {
        let previous = ViewSnapshot::from_state(&AppState::default());
        let mut next = previous.clone();
        next.domain.time.time_str = "12:01".to_string();

        let groups = SyncGroups::between(&previous, &next);

        assert!(groups.time);
        assert!(groups.prayer);
    }

    #[test]
    fn prayer_schedule_cache_reuses_same_minute() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let key = PrayerScheduleKey {
            city: 0,
            date,
            minute: 12 * 60,
        };
        let mut cache = PrayerCache::default();

        assert!(cache.needs_refresh(key.clone()));
        for _ in 0..100 {
            assert!(!cache.needs_refresh(key.clone()));
        }
    }

    #[test]
    fn art_cache_skips_repeated_empty_and_missing_paths() {
        let mut cache = ArtCache::default();
        assert!(matches!(cache.resolve(""), ArtUpdate::Update(None)));
        for _ in 0..100 {
            assert!(matches!(cache.resolve(""), ArtUpdate::Unchanged));
        }

        let missing = std::env::temp_dir().join(format!(
            "notchde-art-cache-missing-{}.png",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&missing);
        let missing = missing.to_str().expect("utf8 path");
        assert!(matches!(cache.resolve(missing), ArtUpdate::Update(None)));
        for _ in 0..100 {
            assert!(matches!(cache.resolve(missing), ArtUpdate::Unchanged));
        }
    }

    #[test]
    fn art_key_tracks_file_changes() {
        let path =
            std::env::temp_dir().join(format!("notchde-art-key-test-{}.png", std::process::id()));
        std::fs::write(&path, b"one").expect("write first art");
        let first = art_key(path.to_str().expect("utf8 path")).expect("first key");

        std::fs::write(&path, b"second").expect("write second art");
        let second = art_key(path.to_str().expect("utf8 path")).expect("second key");

        assert_ne!(first, second);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn art_cache_evicts_beyond_capacity() {
        let dir =
            std::env::temp_dir().join(format!("notchde-art-cache-capacity-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create art cache test dir");
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("icons")
            .join("play.svg");
        let mut cache = ArtCache::default();

        for index in 0..=ART_CACHE_CAPACITY {
            let path = dir.join(format!("art-{index}.svg"));
            std::fs::copy(&source, &path).expect("copy art fixture");
            let path = path.to_str().expect("utf8 path");
            assert!(matches!(cache.resolve(path), ArtUpdate::Update(Some(_))));
        }

        assert_eq!(cache.entries.len(), ART_CACHE_CAPACITY);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn source_cache_reuses_identical_sources() {
        let mut cache = SourceCache::default();
        let sources = vec![MediaSource {
            id: "player".to_string(),
            name: "Player".to_string(),
            is_active: true,
            is_playing: false,
        }];

        assert!(cache.changed(&sources));
        assert!(!cache.changed(&sources));
    }
}
