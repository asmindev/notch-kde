use island_core::{reduce, AppEvent, AppState, Effect};
use island_platform::overlap::Rect;
use island_platform::OverlapDetector;
use island_services::{AudioService, BatteryService, CavaService, ClockService, DndService, MediaService};
use island_ui::{MainWindow, ViewModel};
use slint::ComponentHandle;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info};

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
        app_state.time = ClockService::get_current_time();
        app_state.battery = BatteryService::read_battery();
        app_state.is_muted = AudioService::check_muted_sync();

        // Sync initial state to UI
        ViewModel::sync_state_to_ui(&self.window, &app_state);

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
        ctx.on_select_player_source(move |player_id| {
            let _ = tx.send(AppEvent::SelectMediaSource(player_id.to_string()));
        });

        let tx = event_tx.clone();
        ctx.on_pet_clicked(move || {
            let _ = tx.send(AppEvent::PetClicked);
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
        let geom = island_platform::KdeManager::get_primary_screen_geometry();
        let notch_w = 380;
        let notch_x = geom.x + if geom.width > notch_w { ((geom.width - notch_w) / 2) as i32 } else { 0 };
        let notch_y = geom.y;
        let notch_rect = Rect {
            x: notch_x,
            y: notch_y,
            w: notch_w,
            h: 76,
        };
        overlap_detector.start(notch_rect, overlap_tx);

        let tx6 = event_tx.clone();
        tokio_rt.spawn(async move {
            while let Some(overlapping) = overlap_rx.recv().await {
                let _ = tx6.send(AppEvent::WindowOverlapChanged(overlapping));
            }
        });

        // 6. Event Processing and UI Dispatch Thread
        let window_weak_clone = window_weak.clone();
        let mut last_synced_state = app_state.clone();
        let event_tx_effects = event_tx.clone();
        std::thread::spawn(move || {
            while let Some(event) = event_rx.blocking_recv() {
                let effects = reduce(&mut app_state, event);

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
                        Effect::SelectMediaSource(player_id) => {
                            debug!("Switching active media source to: {}", player_id);
                            tokio_rt.spawn(async move {
                                MediaService::select_source(&player_id).await;
                            });
                        }
                        Effect::ScheduleTimer { id, duration_ms, generation } => {
                            if id == "bloom_wide" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::ViewSwitchRequested(island_core::models::IslandView::Wide));
                                });
                            } else if id == "alert_timeout" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::AlertTimeout(generation));
                                });
                            } else if id == "pet_happy_timeout" {
                                let tx_timer = event_tx_effects.clone();
                                tokio_rt.spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                                    let _ = tx_timer.send(AppEvent::PetHappyTimeout(generation));
                                });
                            }
                        }
                        _ => {}
                    }
                }

                // Dirty diff: only dispatch to Slint UI thread if state actually changed
                if app_state != last_synced_state {
                    last_synced_state = app_state.clone();
                    let state_clone = app_state.clone();
                    let w = window_weak_clone.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(win) = w.upgrade() {
                            ViewModel::sync_state_to_ui(&win, &state_clone);
                        }
                    });
                }
            }
        });

        // 7. Show Window and Run Slint GUI event loop
        self.window.show()?;
        self.window.run()
    }
}
