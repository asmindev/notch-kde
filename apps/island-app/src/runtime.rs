use island_core::{reduce, AppEvent, AppState, Effect, PersistedSettings, SettingsTab};
use island_platform::overlap::Rect;
use island_platform::{KdeManager, OverlapDetector, PlatformWindow};
use island_services::{
    AudioService, BatteryService, CavaService, ClockService, DndService, MediaService,
    NotificationService, PrayerService, ScreenSaverService,
};
use island_ui::{IslandContext, MainWindow, SettingsWindow, ViewModel};
use crate::input_region;
use crate::pet3d;
use crate::settings_store::SettingsStore;
use slint::ComponentHandle;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

const COMPACT_NOTCH_WIDTH: u32 = 156;
const COMPACT_NOTCH_HEIGHT: u32 = 30;
const ISLAND_WINDOW_WIDTH: u32 = 480;

fn compact_notch_rect(geom: &island_platform::kde::ScreenGeometry) -> Rect {
    Rect {
        x: geom.x
            + if geom.width > COMPACT_NOTCH_WIDTH {
                ((geom.width - COMPACT_NOTCH_WIDTH) / 2) as i32
            } else {
                0
            },
        y: geom.y,
        w: COMPACT_NOTCH_WIDTH,
        h: COMPACT_NOTCH_HEIGHT,
    }
}

fn window_x_for(geom: &island_platform::kde::ScreenGeometry) -> i32 {
    geom.x + (geom.width.saturating_sub(ISLAND_WINDOW_WIDTH) / 2) as i32
}

pub struct AppRuntime {
    window: MainWindow,
}

impl AppRuntime {
    pub fn new(window: MainWindow) -> Self {
        Self { window }
    }

    pub fn run(self) -> Result<(), slint::PlatformError> {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AppEvent>();

        // 1. Initial State
        let mut app_state = AppState::default();
        SettingsStore::load().apply_to_state(&mut app_state);
        app_state.time = ClockService::get_current_time();
        app_state.battery = BatteryService::read_battery();
        app_state.is_muted = AudioService::check_muted_sync();

        // Personalized welcome greeting
        let username = std::env::var("USER").unwrap_or_else(|_| "User".to_string());
        let mut chars = username.chars();
        let formatted_user = match chars.next() {
            None => "User".to_string(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        };
        app_state.welcome_title = format!("Welcome back, {}!", formatted_user);

        let now = chrono::Local::now();
        let day_name = now.format("%A").to_string();
        let hour = now.format("%H").to_string().parse::<u32>().unwrap_or(12);

        let greeting_sub = if hour < 12 {
            format!("Have a wonderful {} morning ✨", day_name)
        } else if hour < 17 {
            format!("Have a wonderful {} afternoon ☀️", day_name)
        } else if hour < 21 {
            format!("Good evening! Enjoy your {} 🌆", day_name)
        } else {
            "Good night! Rest and recharge 🌙".to_string()
        };
        app_state.welcome_subtitle = greeting_sub;

        let (prayer_tx, prayer_rx) = tokio::sync::watch::channel(app_state.prayer);
        let settings_window = SettingsWindow::new()?;

        // Sync initial state to UI
        ViewModel::sync_state_to_ui(&self.window, &app_state);
        ViewModel::sync_settings_to_ui(&settings_window, &app_state);

        let input_region = input_region::InputRegionHandle::spawn();
        let initial_input_rect = input_region::rect_for_state(&app_state);

        // Install the offscreen Momo renderer before the window is shown.
        let pet3d = pet3d::install(&self.window);
        pet3d.update_from_state(&app_state);
        pet3d.tick();

        // The Settings window has its own WGPU context, so it needs its own renderer.
        let settings_pet3d = pet3d::install_settings(&settings_window);
        settings_pet3d.update_from_state(&app_state);
        settings_pet3d.tick();

        // 2. Wire UI Callbacks to Event Channel
        let tx = event_tx.clone();
        let hover_gen = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let hover_gen_cb = hover_gen.clone();
        self.window.on_hover_changed(move |hovered| {
            if hovered {
                hover_gen_cb.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let _ = tx.send(AppEvent::UserHoverChanged(true));
            } else {
                let current_gen = hover_gen_cb.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                let tx_clone = tx.clone();
                let gen_check = hover_gen_cb.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(160));
                    if gen_check.load(std::sync::atomic::Ordering::SeqCst) == current_gen {
                        let _ = tx_clone.send(AppEvent::UserHoverChanged(false));
                    }
                });
            }
        });

        let tx = event_tx.clone();
        self.window.on_island_clicked(move || {
            let _ = tx.send(AppEvent::IslandClicked);
        });

        // Wire IslandContext (React-style Context) callbacks
        let ctx = self.window.global::<island_ui::IslandContext>();

        let tx = event_tx.clone();
        ctx.on_collapse_clicked(move || {
            let _ = tx.send(AppEvent::CollapseClicked);
        });

        let tx = event_tx.clone();
        ctx.on_play_pause(move || {
            let _ = tx.send(AppEvent::PlayPauseClicked);
        });

        let tx = event_tx.clone();
        ctx.on_next_track(move || {
            let _ = tx.send(AppEvent::NextTrackClicked);
        });

        let tx = event_tx.clone();
        ctx.on_prev_track(move || {
            let _ = tx.send(AppEvent::PreviousTrackClicked);
        });

        let tx = event_tx.clone();
        ctx.on_toggle_mute(move || {
            let _ = tx.send(AppEvent::MuteToggleClicked);
        });

        let tx = event_tx.clone();
        ctx.on_toggle_dnd(move || {
            let _ = tx.send(AppEvent::DndToggleClicked);
        });

        let tx = event_tx.clone();
        ctx.on_toggle_swap(move || {
            let _ = tx.send(AppEvent::ToggleSwapPetArt);
        });

        let tx = event_tx.clone();
        ctx.on_toggle_source_selector(move || {
            let _ = tx.send(AppEvent::ToggleSourceSelector);
        });

        let tx = event_tx.clone();
        ctx.on_toggle_prayer_panel(move || {
            let _ = tx.send(AppEvent::TogglePrayerPanel);
        });

        let tx = event_tx.clone();
        ctx.on_select_player_source(move |player_id| {
            let _ = tx.send(AppEvent::SelectMediaSource(player_id.to_string()));
        });

        let tx = event_tx.clone();
        ctx.on_pet_clicked(move || {
            let _ = tx.send(AppEvent::PetClicked);
        });

        let tx = event_tx.clone();
        ctx.on_mochi_clicked(move || {
            let _ = tx.send(AppEvent::MochiSlapped);
        });

        let tx = event_tx.clone();
        ctx.on_cycle_blob_character(move || {
            let _ = tx.send(AppEvent::CycleBlobCharacter);
        });

        let tx = event_tx.clone();
        ctx.on_cycle_blob_palette(move || {
            let _ = tx.send(AppEvent::CycleBlobPalette);
        });

        let tx = event_tx.clone();
        ctx.on_cycle_blob_expression(move || {
            let _ = tx.send(AppEvent::CycleBlobExpression);
        });

        let tx = event_tx.clone();
        ctx.on_select_blob_character(move |index| {
            let _ = tx.send(AppEvent::SetBlobCharacter(index.max(0) as u32));
        });

        let tx = event_tx.clone();
        ctx.on_select_blob_palette(move |index| {
            let _ = tx.send(AppEvent::SetBlobPalette(index.max(0) as u32));
        });

        let tx = event_tx.clone();
        ctx.on_select_blob_expression(move |index| {
            let _ = tx.send(AppEvent::SetBlobExpression(index.max(0) as u32));
        });

        let tx = event_tx.clone();
        ctx.on_randomize_avatar(move || {
            let _ = tx.send(AppEvent::RandomizeAvatar);
        });

        let tx = event_tx.clone();
        ctx.on_toggle_avatar_random(move || {
            let _ = tx.send(AppEvent::ToggleAvatarRandom);
        });

        let tx = event_tx.clone();
        ctx.on_open_settings(move || {
            let _ = tx.send(AppEvent::OpenSettings);
        });

        // Settings window callbacks use the same event channel as the island.
        let settings_ctx = settings_window.global::<IslandContext>();
        let tx = event_tx.clone();
        settings_ctx.on_close_settings(move || {
            let _ = tx.send(AppEvent::CloseSettings);
        });
        let tx = event_tx.clone();
        settings_ctx.on_select_settings_tab(move |index| {
            let _ = tx.send(AppEvent::SetSettingsTab(SettingsTab::from_index(index)));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_cursor_gaze(move |enabled| {
            let _ = tx.send(AppEvent::SetCursorGaze(enabled));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_idle_acting(move |enabled| {
            let _ = tx.send(AppEvent::SetIdleActing(enabled));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_click_stunts(move |enabled| {
            let _ = tx.send(AppEvent::SetClickStunts(enabled));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_notification_reactions(move |enabled| {
            let _ = tx.send(AppEvent::SetNotificationReactions(enabled));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_music_reactions(move |enabled| {
            let _ = tx.send(AppEvent::SetMusicReactions(enabled));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_sleepy_on_low_battery(move |enabled| {
            let _ = tx.send(AppEvent::SetSleepyOnLowBattery(enabled));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_shadow_enabled(move |enabled| {
            let _ = tx.send(AppEvent::SetShadowEnabled(enabled));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_shadow_intensity(move |intensity| {
            let _ = tx.send(AppEvent::SetShadowIntensity(intensity.max(0) as u32));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_shadow_radius(move |radius| {
            let _ = tx.send(AppEvent::SetShadowRadius(radius.max(0) as u32));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_shadow_offset_x(move |offset| {
            let _ = tx.send(AppEvent::SetShadowOffsetX(offset));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_shadow_offset_y(move |offset| {
            let _ = tx.send(AppEvent::SetShadowOffsetY(offset));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_prayer_enabled(move |enabled| {
            let _ = tx.send(AppEvent::SetPrayerEnabled(enabled));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_prayer_city(move |index| {
            let _ = tx.send(AppEvent::SetPrayerCity(index.max(0) as u32));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_prayer_reminder_minutes(move |minutes| {
            let _ = tx.send(AppEvent::SetPrayerReminderMinutes(minutes.max(0) as u32));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_media_visual_mode(move |mode| {
            let visual_mode = if mode == 1 {
                island_core::models::MusicVisualMode::PetPrimary
            } else {
                island_core::models::MusicVisualMode::ArtworkPrimary
            };
            let _ = tx.send(AppEvent::SetMediaVisualMode(visual_mode));
        });
        let tx = event_tx.clone();
        settings_ctx.on_reset_avatar(move || {
            let _ = tx.send(AppEvent::ResetAvatar);
        });
        let tx = event_tx.clone();
        settings_ctx.on_reset_settings(move || {
            let _ = tx.send(AppEvent::ResetSettings);
        });
        let tx = event_tx.clone();
        settings_ctx.on_select_blob_character(move |index| {
            let _ = tx.send(AppEvent::SetBlobCharacter(index.max(0) as u32));
        });
        let tx = event_tx.clone();
        settings_ctx.on_select_blob_palette(move |index| {
            let _ = tx.send(AppEvent::SetBlobPalette(index.max(0) as u32));
        });
        let tx = event_tx.clone();
        settings_ctx.on_select_blob_expression(move |index| {
            let _ = tx.send(AppEvent::SetBlobExpression(index.max(0) as u32));
        });
        let tx = event_tx.clone();
        settings_ctx.on_randomize_avatar(move || {
            let _ = tx.send(AppEvent::RandomizeAvatar);
        });
        let tx = event_tx.clone();
        settings_ctx.on_toggle_avatar_random(move || {
            let _ = tx.send(AppEvent::ToggleAvatarRandom);
        });
        let tx = event_tx.clone();
        settings_ctx.on_pet_clicked(move || {
            let _ = tx.send(AppEvent::PetClicked);
        });
        let tx = event_tx.clone();
        settings_ctx.on_mochi_clicked(move || {
            let _ = tx.send(AppEvent::MochiSlapped);
        });

        let settings_close_tx = event_tx.clone();
        settings_window.window().on_close_requested(move || {
            let _ = settings_close_tx.send(AppEvent::CloseSettings);
            slint::CloseRequestResponse::HideWindow
        });

        let pet3d_cursor = pet3d.clone();
        let cursor_window = self.window.as_weak();
        ctx.on_cursor_moved(move |x, y| {
            pet3d_cursor.set_gaze(x, y);
            if pet3d_cursor.tick().dirty
                && let Some(win) = cursor_window.upgrade()
            {
                win.window().request_redraw();
            }
        });

        // 3. Start Tokio Async Runtime for Background Services
        let window_weak = self.window.as_weak();
        let tokio_rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to initialize Tokio runtime");

        let event_tx_services = event_tx.clone();

        tokio_rt.spawn(async move {
            // Clock Service
            let (clock_tx, mut clock_rx) = mpsc::unbounded_channel();
            ClockService::start(clock_tx);
            let tx1 = event_tx_services.clone();
            tokio::spawn(async move {
                while let Some(time_info) = clock_rx.recv().await {
                    let _ = tx1.send(AppEvent::Tick(time_info));
                }
            });

            // Battery Service
            let (bat_tx, mut bat_rx) = mpsc::unbounded_channel();
            BatteryService::start(bat_tx);
            let tx2 = event_tx_services.clone();
            tokio::spawn(async move {
                while let Some(bat_state) = bat_rx.recv().await {
                    let _ = tx2.send(AppEvent::BatteryUpdated(bat_state));
                }
            });

            // Media Service
            let (media_tx, mut media_rx) = mpsc::unbounded_channel();
            MediaService::start(media_tx);
            let tx3 = event_tx_services.clone();
            tokio::spawn(async move {
                while let Some(update) = media_rx.recv().await {
                    let _ = tx3.send(AppEvent::MediaUpdated(update.track));
                    let _ = tx3.send(AppEvent::MediaSourcesUpdated(update.sources));
                }
            });

            // Desktop notifications (session bus monitor)
            let (notif_tx, mut notif_rx) = mpsc::unbounded_channel();
            tokio::spawn(NotificationService::listen(notif_tx));
            let tx_notif = event_tx_services.clone();
            tokio::spawn(async move {
                while notif_rx.recv().await.is_some() {
                    let _ = tx_notif.send(AppEvent::NotificationReceived);
                }
            });

            // Prayer Service
            let (prayer_alert_tx, mut prayer_alert_rx) = mpsc::unbounded_channel();
            PrayerService::start(prayer_rx, prayer_alert_tx);
            let tx_prayer = event_tx_services.clone();
            tokio::spawn(async move {
                while let Some(alert) = prayer_alert_rx.recv().await {
                    let _ = tx_prayer.send(AppEvent::PrayerAlert(alert));
                }
            });

            // DND Service (session bus)
            let (dnd_tx, mut dnd_rx) = mpsc::unbounded_channel();
            DndService::start(dnd_tx);
            let tx_dnd = event_tx_services.clone();
            tokio::spawn(async move {
                while let Some(is_dnd) = dnd_rx.recv().await {
                    let _ = tx_dnd.send(AppEvent::DndUpdated(is_dnd));
                }
            });

            // Audio Mute Service (pactl event stream)
            let (audio_tx, mut audio_rx) = mpsc::unbounded_channel();
            AudioService::start(audio_tx);
            let tx_audio = event_tx_services.clone();
            tokio::spawn(async move {
                while let Some(is_muted) = audio_rx.recv().await {
                    let _ = tx_audio.send(AppEvent::AudioMuteUpdated(is_muted));
                }
            });

            // ScreenSaver / Lockscreen Service (Detects unlock after lockscreen)
            let (unlock_tx, mut unlock_rx) = mpsc::unbounded_channel();
            ScreenSaverService::start(unlock_tx);
            let tx_unlock = event_tx_services.clone();
            tokio::spawn(async move {
                while let Some(_) = unlock_rx.recv().await {
                    let _ = tx_unlock.send(AppEvent::ScreenUnlocked);
                }
            });

            // Welcome Timeout (Phase 4): Transition after 3.5 seconds
            let tx4 = event_tx_services.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(3500)).await;
                info!("Welcome hold timeout triggered -> morphing to standard view");
                let _ = tx4.send(AppEvent::WelcomeTimeout);
            });
        });

        // 4. Start CAVA Audio Spectrum Visualizer
        let cava_service = Arc::new(CavaService::new());
        let (cava_tx, mut cava_rx) = mpsc::unbounded_channel();
        cava_service.start(cava_tx);

        let tx5 = event_tx.clone();
        tokio_rt.spawn(async move {
            while let Some(bars) = cava_rx.recv().await {
                let _ = tx5.send(AppEvent::SpectrumUpdated(bars));
            }
        });

        // 5. Start Wayland Overlap Detector
        let overlap_detector = Arc::new(OverlapDetector::new());
        let (overlap_tx, mut overlap_rx) = mpsc::unbounded_channel();
        let geom = KdeManager::get_primary_screen_geometry();
        let notch_rect = compact_notch_rect(&geom);
        overlap_detector.start(notch_rect, overlap_tx);

        let screen_watcher = overlap_detector.clone();
        let output_config = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .map(|home| home.join(".config/kwinoutputconfig.json"));
        std::thread::spawn(move || {
            let mut last_mtime = output_config
                .as_ref()
                .and_then(|path| std::fs::metadata(path).ok())
                .and_then(|meta| meta.modified().ok());
            let mut last_periodic = std::time::Instant::now();

            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let mtime = output_config
                    .as_ref()
                    .and_then(|path| std::fs::metadata(path).ok())
                    .and_then(|meta| meta.modified().ok());
                let changed = mtime != last_mtime;
                let periodic = last_periodic.elapsed() >= std::time::Duration::from_secs(60);
                if !changed && !periodic {
                    continue;
                }
                last_mtime = mtime;
                last_periodic = std::time::Instant::now();

                let Some(geom) = KdeManager::try_get_primary_screen_geometry() else {
                    last_mtime = None;
                    continue;
                };
                let window_x = window_x_for(&geom);
                match PlatformWindow::ensure_kwin_rules(window_x, geom.y, geom.screen_index) {
                    Ok(true) => {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                    Ok(false) => {}
                    Err(err) => warn!("Failed to update KWin rule after screen change: {err}"),
                }
                screen_watcher.update_rect(compact_notch_rect(&geom));
            }
        });

        let tx6 = event_tx.clone();
        tokio_rt.spawn(async move {
            while let Some(overlapping) = overlap_rx.recv().await {
                let _ = tx6.send(AppEvent::WindowOverlapChanged(overlapping));
            }
        });

        // 6. Avatar is static by default. Auto-random only runs after the user
        // enables it from the avatar picker.
        let variation_tx = event_tx.clone();
        tokio_rt.spawn(async move {
            let mut random = tokio::time::interval(tokio::time::Duration::from_secs(5));
            random.tick().await;
            loop {
                random.tick().await;
                let _ = variation_tx.send(AppEvent::AvatarRandomTick);
            }
        });

        // 7. Event Processing and UI Dispatch Thread
        let window_weak_clone = window_weak.clone();
        let settings_weak_events = settings_window.as_weak();
        let pet3d_events = pet3d.clone();
        let settings_pet3d_events = settings_pet3d.clone();
        let mut last_synced_state = app_state.clone();
        let mut last_settings_open = app_state.settings_open;
        let mut last_prayer = app_state.prayer;
        let event_tx_effects = event_tx.clone();
        let input_region_events = input_region.clone();
        let mut last_input_rect = initial_input_rect;
        std::thread::spawn(move || {
            while let Some(event) = event_rx.blocking_recv() {
                let effects = reduce(&mut app_state, event);

                if app_state.prayer != last_prayer {
                    last_prayer = app_state.prayer;
                    let _ = prayer_tx.send(app_state.prayer);
                }

                let input_rect = input_region::rect_for_state(&app_state);
                if input_rect != last_input_rect {
                    input_region_events.set_rect(input_rect);
                    last_input_rect = input_rect;
                }

                // Execute any side effects
                for effect in effects {
                    match effect {
                        Effect::PlayPause => {
                            tokio_rt.spawn(async { MediaService::play_pause().await; });
                        }
                        Effect::NextTrack => {
                            tokio_rt.spawn(async { MediaService::next().await; });
                        }
                        Effect::PreviousTrack => {
                            tokio_rt.spawn(async { MediaService::previous().await; });
                        }
                        Effect::ToggleMute => {
                            debug!("Toggling system mute");
                            tokio_rt.spawn(async { AudioService::toggle_mute().await; });
                        }
                        Effect::ToggleDnd => {
                            debug!("Toggling notification Do-Not-Disturb");
                            tokio_rt.spawn(async { DndService::toggle_dnd().await; });
                        }
                        Effect::PersistSettings => {
                            SettingsStore::save(&PersistedSettings::from_state(&app_state));
                        }
                        Effect::SelectMediaSource(player_id) => {
                            debug!("Switching active media source to: {}", player_id);
                            tokio_rt.spawn(async move {
                                MediaService::select_source(&player_id).await;
                            });
                        }
                        Effect::ScheduleTimer { id, duration_ms, generation } => {
                            if id == "overlap_reveal" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::OverlapRevealTimeout(generation));
                                });
                            } else if id == "overlap_dismiss_compact" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::OverlapDismissCompactTimeout(generation));
                                });
                            } else if id == "overlap_dismiss_hide" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::OverlapDismissHideTimeout(generation));
                                });
                            } else if id == "collapse_compact" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::CollapseCompactTimeout(generation));
                                });
                            } else if id == "alert_timeout" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::AlertTimeout(generation));
                                });
                            } else if id == "mochi_annoyed_timeout" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::MochiAnnoyedTimeout(generation));
                                });
                            } else if id == "mochi_dizzy_timeout" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::MochiDizzyTimeout(generation));
                                });
                            } else if id == "mochi_happy_timeout" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::MochiHappyTimeout(generation));
                                });
                            } else if id == "mochi_slap_reset" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::MochiSlapReset(generation));
                                });
                            } else if id == "pet_happy_timeout" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::PetHappyTimeout(generation));
                                });
                            } else if id == "pet_dizzy_timeout" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::PetDizzyTimeout(generation));
                                });
                            } else if id == "pet_poke_reset" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::PetPokeReset(generation));
                                });
                            } else if id == "welcome_timeout" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::WelcomeTimeout);
                                });
                            }
                        }
                    }
                }

                // Dirty diff: only dispatch to Slint UI thread if state actually changed
                if app_state != last_synced_state {
                    last_synced_state = app_state.clone();
                    let state_clone = app_state.clone();
                    let w = window_weak_clone.clone();
                    let settings_w = settings_weak_events.clone();
                    let pet3d = pet3d_events.clone();
                    let settings_pet3d = settings_pet3d_events.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(win) = w.upgrade() {
                            ViewModel::sync_state_to_ui(&win, &state_clone);
                            pet3d.update_from_state(&state_clone);
                            if pet3d.tick().dirty {
                                win.window().request_redraw();
                            }
                            if let Some(settings) = settings_w.upgrade() {
                                ViewModel::sync_settings_to_ui(&settings, &state_clone);
                                settings_pet3d.update_from_state(&state_clone);
                                if settings.window().is_visible() {
                                    settings_pet3d.tick();
                                    settings.window().request_redraw();
                                }
                            }
                        }
                    });
                }

                if app_state.settings_open != last_settings_open {
                    last_settings_open = app_state.settings_open;
                    let settings_w = settings_weak_events.clone();
                    let state_clone = app_state.clone();
                    let settings_pet3d = settings_pet3d_events.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(settings) = settings_w.upgrade() {
                            if state_clone.settings_open {
                                ViewModel::sync_settings_to_ui(&settings, &state_clone);
                                settings_pet3d.update_from_state(&state_clone);
                                settings_pet3d.tick();
                                settings.window().request_redraw();
                                let _ = settings.show();
                            } else {
                                let _ = settings.hide();
                            }
                        }
                    });
                }
            }
        });

        // 8. Show Window and Run Slint GUI event loop
        self.window.show()?;
        input_region::schedule_init(
            self.window.as_weak(),
            input_region,
            initial_input_rect,
            20,
        );
        let _pet3d_timer = pet3d::start_timer(pet3d.clone(), self.window.as_weak());
        let _settings_pet3d_timer =
            pet3d::start_settings_timer(settings_pet3d.clone(), settings_window.as_weak());
        self.window.run()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use island_platform::kde::ScreenGeometry;

    #[test]
    fn compact_notch_rect_is_centered_and_compact() {
        let geom = ScreenGeometry {
            x: 100,
            y: 20,
            width: 1516,
            height: 947,
            screen_index: 0,
        };

        let rect = compact_notch_rect(&geom);

        assert_eq!(rect.x, 780);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.w, 156);
        assert_eq!(rect.h, 30);
    }

    #[test]
    fn compact_notch_rect_handles_narrow_screen() {
        let geom = ScreenGeometry {
            x: 0,
            y: 0,
            width: 120,
            height: 800,
            screen_index: 0,
        };

        let rect = compact_notch_rect(&geom);

        assert_eq!(rect.x, 0);
        assert_eq!(rect.w, 156);
    }
}
