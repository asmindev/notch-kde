mod effects;
mod timers;

use crate::config::AppConfig;
use crate::input_region;
use crate::pet3d;
use crate::render_stats::RenderStats;
use crate::settings_store::SettingsStore;
use effects::EffectExecutor;
use island_core::{
    AppEvent, AppState, CompactModeStyle, Effect, PersistedSettings, PetRenderer, SettingsTab,
    TimerId, reduce, render_policy,
};
use island_platform::kde::ScreenGeometry;
use island_platform::{KdeManager, OverlapDetector, PlatformWindow};
use island_services::{
    AudioService, BatteryService, CavaService, ClockService, DndService, DomainEvent, MediaService,
    NotificationService, PrayerService, ScreenSaverService, ServiceContext, ServiceSupervisor,
};
use island_ui::{IslandContext, MainWindow, ModuleUiRegistry, SettingsWindow, ViewModel};
use slint::ComponentHandle;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, warn};

pub struct AppRuntime {
    window: MainWindow,
    config: AppConfig,
    settings: PersistedSettings,
}

impl AppRuntime {
    pub fn new(window: MainWindow, config: AppConfig, settings: PersistedSettings) -> Self {
        Self {
            window,
            config,
            settings,
        }
    }

    pub fn run(self) -> Result<(), slint::PlatformError> {
        let config = self.config;
        let window_spec = config.window;
        let settings_path = config.settings_path();
        let initial_settings = self.settings;
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AppEvent>();

        // 1. Initial State
        let mut app_state = AppState::default();
        app_state.settings.pet_renderer = config.pet.renderer;
        initial_settings.apply_to_state(&mut app_state);
        app_state
            .view
            .engine
            .set_dodge_enabled(config.dodge_enabled);
        app_state.domain.time = ClockService::get_current_time();
        app_state.domain.battery = BatteryService::read_battery();
        app_state.domain.is_muted = AudioService::check_muted_sync();

        for issue in ModuleUiRegistry::default().validate(app_state.view.engine.registry()) {
            warn!("{issue}; falling back to system");
        }

        // Personalized welcome greeting
        let username = std::env::var("USER").unwrap_or_else(|_| "User".to_string());
        let mut chars = username.chars();
        let formatted_user = match chars.next() {
            None => "User".to_string(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        };
        app_state.presentation.welcome_title = format!("Welcome back, {}!", formatted_user);

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
        app_state.presentation.welcome_subtitle = greeting_sub;

        let (prayer_tx, prayer_rx) = tokio::sync::watch::channel(app_state.settings.prayer);
        let settings_window = SettingsWindow::new()?;

        // Sync initial state to UI
        let render_stats = Arc::new(RenderStats::default());
        let initial_snapshot = app_state.snapshot();
        let initial_policy = render_policy(&app_state);
        render_stats.ui_sync();
        ViewModel::sync_state_to_ui(&self.window, &initial_snapshot, initial_policy);
        if !initial_policy.spectrum {
            render_stats.ui_sync();
            ViewModel::clear_spectrum_to_ui(&self.window);
        }
        ViewModel::sync_settings_to_ui(&settings_window, &initial_snapshot);

        let input_region = input_region::InputRegionHandle::spawn();
        let initial_input_rect = input_region::rect_for_state(&app_state, &window_spec);

        // Install the flat renderer first. 3D initializes a standalone WGPU
        // device so both modes can switch at runtime without changing the
        // OpenGL window backend.
        let pet3d = pet3d::install(&self.window);
        pet3d.set_render_active(initial_policy.pet_3d);
        pet3d.update_from_state(&initial_snapshot);
        if initial_policy.pet_3d {
            render_stats.pet_tick();
            pet3d.tick();
        }
        if app_state.settings.pet_renderer.is_3d() {
            if let Some(image) = pet3d.switch_renderer(app_state.settings.pet_renderer) {
                let ctx = self.window.global::<IslandContext>();
                ctx.set_pet_3d_image(image);
                ctx.set_pet_3d_ready(true);
            }
        }

        let settings_pet3d = pet3d::install_settings(&settings_window);
        settings_pet3d.update_from_state(&initial_snapshot);
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
                let current_gen =
                    hover_gen_cb.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
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
        settings_ctx.on_set_pet_renderer(move |index| {
            let renderer = if index == 1 {
                PetRenderer::TwoD
            } else {
                PetRenderer::ThreeD
            };
            let _ = tx.send(AppEvent::SetPetRenderer(renderer));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_compact_mode_style(move |index| {
            let style = CompactModeStyle::from_index(index);
            let _ = tx.send(AppEvent::SetCompactModeStyle(style));
        });
        let tx = event_tx.clone();
        settings_ctx.on_set_cava_bg_opacity(move |val| {
            let _ = tx.send(AppEvent::SetCavaBgOpacity(val.clamp(0, 100) as u32));
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
        let render_stats_cursor = render_stats.clone();
        let cursor_window = self.window.as_weak();
        ctx.on_cursor_moved(move |x, y| {
            if !pet3d_cursor.is_render_active() {
                return;
            }
            pet3d_cursor.set_gaze(x, y);
            render_stats_cursor.pet_tick();
            if pet3d_cursor.tick().dirty
                && let Some(win) = cursor_window.upgrade()
            {
                render_stats_cursor.pet_redraw();
                win.window().request_redraw();
            }
        });

        // 3. Start Tokio Async Runtime for Background Services
        let window_weak = self.window.as_weak();
        let tokio_rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to initialize Tokio runtime");

        let render_stats_log = render_stats.clone();
        tokio_rt.spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            interval.tick().await;
            let mut previous = render_stats_log.snapshot();
            let mut previous_at = std::time::Instant::now();
            loop {
                interval.tick().await;
                let now = render_stats_log.snapshot();
                let elapsed = previous_at.elapsed().as_secs_f64();
                let rates = now.rates_since(previous, elapsed);
                debug!(
                    pet_ticks_per_sec = rates.pet_ticks_per_sec,
                    pet_redraws_per_sec = rates.pet_redraws_per_sec,
                    ui_syncs_per_sec = rates.ui_syncs_per_sec,
                    "render activity"
                );
                previous = now;
                previous_at = std::time::Instant::now();
            }
        });

        let media_service = MediaService::new();
        let (domain_tx, mut domain_rx) = mpsc::unbounded_channel();
        let mut supervisor = ServiceSupervisor::new(std::time::Duration::from_secs(3));
        let service_shutdown = supervisor.shutdown_signal();
        let service_ctx =
            ServiceContext::new(tokio_rt.handle().clone(), domain_tx, service_shutdown);

        let services = config.services;
        if services.clock {
            supervisor.register(Box::new(ClockService));
        }
        if services.battery {
            supervisor.register(Box::new(BatteryService));
        }
        if services.media {
            supervisor.register(Box::new(media_service.clone()));
        }
        if services.notifications {
            supervisor.register(Box::new(NotificationService::new()));
        }
        if services.prayer {
            supervisor.register(Box::new(PrayerService::new(prayer_rx)));
        }
        if services.dnd {
            supervisor.register(Box::new(DndService));
        }
        if services.audio {
            supervisor.register(Box::new(AudioService));
        }
        if services.screen_saver {
            supervisor.register(Box::new(ScreenSaverService));
        }
        if config.cava.enabled {
            supervisor.register(Box::new(CavaService::new(config.cava.config_path.clone())));
        }
        supervisor.start_all(&service_ctx);

        let tx_domain = event_tx.clone();
        tokio_rt.spawn(async move {
            while let Some(event) = domain_rx.recv().await {
                let app_events = match event {
                    DomainEvent::Tick(time) => vec![AppEvent::Tick(time)],
                    DomainEvent::Battery(battery) => vec![AppEvent::BatteryUpdated(battery)],
                    DomainEvent::Media(update) => vec![
                        AppEvent::MediaUpdated(update.track),
                        AppEvent::MediaSourcesUpdated(update.sources),
                    ],
                    DomainEvent::Dnd(is_dnd) => vec![AppEvent::DndUpdated(is_dnd)],
                    DomainEvent::Muted(is_muted) => vec![AppEvent::AudioMuteUpdated(is_muted)],
                    DomainEvent::Notification => vec![AppEvent::NotificationReceived],
                    DomainEvent::Prayer(alert) => vec![AppEvent::PrayerAlert(alert)],
                    DomainEvent::ScreenUnlocked => vec![AppEvent::ScreenUnlocked],
                    DomainEvent::Spectrum(bars) => vec![AppEvent::SpectrumUpdated(bars)],
                };
                for event in app_events {
                    if tx_domain.send(event).is_err() {
                        return;
                    }
                }
            }
        });

        // 5. Start Wayland Overlap Detector
        let overlap_detector = Arc::new(OverlapDetector::new());
        let (overlap_tx, mut overlap_rx) = mpsc::unbounded_channel();
        let geom = KdeManager::try_get_primary_screen_geometry().unwrap_or_else(|| {
            warn!("kscreen-doctor/qdbus unavailable; using fallback screen geometry");
            ScreenGeometry::fallback()
        });
        let notch_rect = window_spec.compact_notch_rect(&geom);
        overlap_detector.start(notch_rect, overlap_tx, config.dodge_enabled);

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
                let window_x = window_spec.window_x(&geom);
                match PlatformWindow::ensure_kwin_rules(window_x, geom.y, geom.screen_index) {
                    Ok(true) => {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                    Ok(false) => {}
                    Err(err) => warn!("Failed to update KWin rule after screen change: {err}"),
                }
                screen_watcher.update_rect(window_spec.compact_notch_rect(&geom));
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
        let render_stats_events = render_stats.clone();
        let settings_pet3d_events = settings_pet3d.clone();
        let mut last_synced_state = app_state.clone();
        let mut last_spectrum_active = initial_policy.spectrum;
        let mut last_settings_open = app_state.presentation.settings_open;
        let mut last_prayer = app_state.settings.prayer;
        let event_tx_effects = event_tx.clone();
        let timer_runtime = tokio_rt.handle().clone();
        let input_region_events = input_region.clone();
        let mut last_input_rect = initial_input_rect;
        let event_thread = std::thread::spawn(move || {
            let mut effect_executor = EffectExecutor::new(
                timer_runtime.clone(),
                event_tx_effects.clone(),
                media_service,
                settings_path.clone(),
            );
            effect_executor.execute(
                Effect::ScheduleTimer {
                    id: TimerId::Welcome,
                    duration_ms: 3500,
                    generation: app_state.presentation.welcome_generation,
                },
                &app_state,
            );

            while let Some(event) = event_rx.blocking_recv() {
                if matches!(event, AppEvent::Shutdown) {
                    break;
                }
                supervisor.refresh_health();
                if matches!(&event, AppEvent::SpectrumUpdated(_)) {
                    let policy = render_policy(&app_state);
                    let _ = reduce(&mut app_state, event);
                    last_synced_state = app_state.clone();
                    if policy.spectrum {
                        let bars = app_state.domain.spectrum.bars;
                        let snapshot = app_state.snapshot();
                        let w = window_weak_clone.clone();
                        let pet3d = pet3d_events.clone();
                        let stats = render_stats_events.clone();
                        stats.ui_sync();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(win) = w.upgrade() {
                                ViewModel::sync_spectrum_to_ui(&win, &bars);
                                if policy.pet_3d {
                                    pet3d.update_from_state(&snapshot);
                                }
                            }
                        });
                    }
                    continue;
                }

                let effects = reduce(&mut app_state, event);

                if app_state.settings.prayer != last_prayer {
                    last_prayer = app_state.settings.prayer;
                    let _ = prayer_tx.send(app_state.settings.prayer);
                }

                let input_rect = input_region::rect_for_state(&app_state, &window_spec);
                if input_rect != last_input_rect {
                    input_region_events.set_rect(input_rect);
                    last_input_rect = input_rect;
                }

                let policy = render_policy(&app_state);
                let spectrum_stopped = last_spectrum_active && !policy.spectrum;
                last_spectrum_active = policy.spectrum;
                pet3d_events.set_render_active(policy.pet_3d);

                for effect in effects {
                    effect_executor.execute(effect, &app_state);
                }

                // Dirty diff: only dispatch to Slint UI thread if state actually changed
                if app_state != last_synced_state {
                    last_synced_state = app_state.clone();
                    let snapshot = app_state.snapshot();
                    let w = window_weak_clone.clone();
                    let settings_w = settings_weak_events.clone();
                    let pet3d = pet3d_events.clone();
                    let settings_pet3d = settings_pet3d_events.clone();
                    let stats = render_stats_events.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(win) = w.upgrade() {
                            stats.ui_sync();
                            ViewModel::sync_state_to_ui(&win, &snapshot, policy);
                            if spectrum_stopped {
                                stats.ui_sync();
                                ViewModel::clear_spectrum_to_ui(&win);
                            }
                            pet3d.update_from_state(&snapshot);
                            let target = snapshot.settings.pet_renderer;
                            if pet3d.target() != target {
                                if let Some(image) = pet3d.switch_renderer(target) {
                                    let ctx = win.global::<IslandContext>();
                                    ctx.set_pet_3d_image(image);
                                    ctx.set_pet_3d_ready(true);
                                }
                            }
                            if policy.pet_3d {
                                stats.pet_tick();
                                if pet3d.tick().dirty {
                                    stats.pet_redraw();
                                    win.window().request_redraw();
                                }
                            }
                            if let Some(settings) = settings_w.upgrade()
                                && (snapshot.presentation.settings_open
                                    || settings.window().is_visible())
                            {
                                ViewModel::sync_settings_to_ui(&settings, &snapshot);
                                settings_pet3d.update_from_state(&snapshot);
                                if settings_pet3d.target() != target {
                                    if let Some(image) = settings_pet3d.switch_renderer(target) {
                                        let ctx = settings.global::<IslandContext>();
                                        ctx.set_pet_3d_image(image);
                                        ctx.set_pet_3d_ready(true);
                                    }
                                }
                                if settings.window().is_visible() {
                                    settings_pet3d.tick();
                                    settings.window().request_redraw();
                                }
                            }
                        }
                    });
                }

                if app_state.presentation.settings_open != last_settings_open {
                    last_settings_open = app_state.presentation.settings_open;
                    let settings_w = settings_weak_events.clone();
                    let state_clone = app_state.clone();
                    let snapshot = app_state.snapshot();
                    let settings_pet3d = settings_pet3d_events.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(settings) = settings_w.upgrade() {
                            if state_clone.presentation.settings_open {
                                ViewModel::sync_settings_to_ui(&settings, &snapshot);
                                settings_pet3d.update_from_state(&snapshot);
                                let target = snapshot.settings.pet_renderer;
                                if settings_pet3d.target() != target {
                                    if let Some(image) = settings_pet3d.switch_renderer(target) {
                                        let ctx = settings.global::<IslandContext>();
                                        ctx.set_pet_3d_image(image);
                                        ctx.set_pet_3d_ready(true);
                                    }
                                }
                                settings_pet3d.tick();
                                settings.window().request_redraw();
                                let _ = settings.show();
                            } else {
                                let _ = settings.hide();
                                let _ = settings_pet3d.switch_renderer(PetRenderer::TwoD);
                            }
                        }
                    });
                }
            }
            effect_executor.shutdown();
            timer_runtime.block_on(supervisor.shutdown());
            SettingsStore::save(&settings_path, &PersistedSettings::from_state(&app_state));
        });

        // 8. Show Window and Run Slint GUI event loop
        self.window.show()?;
        input_region::schedule_init(self.window.as_weak(), input_region, initial_input_rect, 20);
        let _pet3d_timer = pet3d::start_timer(pet3d, self.window.as_weak(), render_stats);
        let _settings_pet3d_timer =
            pet3d::start_settings_timer(settings_pet3d, settings_window.as_weak());
        let result = self.window.run();
        let _ = event_tx.send(AppEvent::Shutdown);
        let _ = event_thread.join();
        result
    }
}
