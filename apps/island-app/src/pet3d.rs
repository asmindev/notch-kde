use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::render_stats::RenderStats;
use island_core::PetRenderer;
use island_core::models::{AlertKind, IslandView};
use island_core::snapshot::ViewSnapshot;
use island_core::state::MochiMoodState;
use island_pet3d::{
    BloubEngine, BloubFlatRenderer, BloubFrame, BloubPalette, BloubRenderer, ExpressionId, ShapeId,
    StateId,
};
use island_ui::{IslandContext, MainWindow, SettingsWindow};
use slint::ComponentHandle;
use slint::wgpu_30::wgpu;
use tracing::{debug, info, warn};

const FRAME_INTERVAL: Duration = Duration::from_millis(33);
const GAZE_TIMEOUT: Duration = Duration::from_millis(2600);
const WELCOME_SWIRL_SECONDS: f32 = 1.3;

#[derive(Clone, Copy, Debug)]
struct Pet3dInput {
    is_singing: bool,
    bass: f32,
    pet_is_happy: bool,
    pet_happy_generation: u64,
    pet_dizzy_generation: u64,
    is_welcome: bool,
    welcome_age: f32,
    welcome_generation: u64,
    is_dizzy: bool,
    is_sleepy: bool,
    is_charging: bool,
    alert_kind: AlertKind,
    notification_generation: u64,
    notification_active: bool,
    is_compact: bool,
    mochi_mood: MochiMoodState,
    blob_character: u32,
    blob_palette: u32,
    blob_expression: u32,
    blob_expression_custom: bool,
    gaze_x: f32,
    gaze_y: f32,
    gaze_active: bool,
    cursor_gaze_enabled: bool,
    idle_acting_enabled: bool,
    notification_reactions_enabled: bool,
}

impl Default for Pet3dInput {
    fn default() -> Self {
        Self {
            is_singing: false,
            bass: 0.0,
            pet_is_happy: false,
            pet_happy_generation: 0,
            pet_dizzy_generation: 0,
            is_welcome: true,
            welcome_age: 0.0,
            welcome_generation: 1,
            is_dizzy: false,
            is_sleepy: false,
            is_charging: false,
            alert_kind: AlertKind::None,
            notification_generation: 0,
            notification_active: false,
            is_compact: false,
            mochi_mood: MochiMoodState::Idle,
            blob_character: 1,
            blob_palette: 0,
            blob_expression: 0,
            blob_expression_custom: false,
            gaze_x: 0.0,
            gaze_y: 0.0,
            gaze_active: false,
            cursor_gaze_enabled: true,
            idle_acting_enabled: true,
            notification_reactions_enabled: true,
        }
    }
}

fn resolve_state(input: &Pet3dInput) -> StateId {
    if input.notification_active {
        StateId::Ping
    } else if input.is_dizzy {
        match input.pet_dizzy_generation % 3 {
            0 => StateId::Orbit,
            1 => StateId::Shake,
            _ => StateId::RotateX,
        }
    } else if input.is_sleepy {
        StateId::Sleep
    } else if input.welcome_age >= 0.0 && input.welcome_age < WELCOME_SWIRL_SECONDS {
        StateId::Swirl
    } else if input.pet_is_happy {
        match input.pet_happy_generation % 4 {
            0 => StateId::Wink,
            1 => StateId::Bounce,
            2 => StateId::Backflip,
            _ => StateId::RotateX,
        }
    } else if input.alert_kind == AlertKind::LowBattery {
        StateId::Alert
    } else if input.alert_kind == AlertKind::Charging {
        StateId::Charge
    } else if input.is_charging {
        StateId::Charging
    } else if input.mochi_mood == MochiMoodState::Annoyed {
        StateId::Alert
    } else if input.is_singing {
        StateId::Orbit
    } else if input.alert_kind != AlertKind::None {
        StateId::Notify
    } else {
        StateId::Idle
    }
}

fn resolve_expression(input: &Pet3dInput, target_state: StateId) -> ExpressionId {
    let selected_expression = ExpressionId::from_index(input.blob_expression as usize);
    if input.is_dizzy {
        ExpressionId::Confus
    } else if input.blob_expression_custom {
        selected_expression
    } else if matches!(target_state, StateId::Backflip | StateId::RotateX) {
        ExpressionId::Surpris
    } else if target_state == StateId::Bounce {
        ExpressionId::Heureux
    } else if matches!(
        target_state,
        StateId::Notify | StateId::Ping | StateId::Charge | StateId::Charging
    ) {
        ExpressionId::Excite
    } else if input.is_sleepy {
        ExpressionId::Somnolent
    } else if input.mochi_mood == MochiMoodState::Annoyed {
        ExpressionId::Colere
    } else if input.pet_is_happy {
        ExpressionId::Heureux
    } else if input.is_singing {
        ExpressionId::Hilare
    } else if target_state == StateId::Swirl {
        ExpressionId::Heureux
    } else {
        selected_expression
    }
}

struct Pet3dShared {
    frame: Option<BloubFrame>,
    palette: BloubPalette,
    dirty: bool,
    ready: bool,
    failed: bool,
}

struct Pet3dTiming {
    last_gaze: Instant,
    last_happy_generation: u64,
    last_dizzy_generation: u64,
    last_welcome_generation: u64,
    last_notification_generation: u64,
    notification_start: Option<Instant>,
    welcome_start: Option<Instant>,
}

pub struct TickResult {
    pub dirty: bool,
}

pub struct Pet3dInner {
    start: Instant,
    input: Mutex<Pet3dInput>,
    shared: Mutex<Pet3dShared>,
    timing: Mutex<Pet3dTiming>,
    engine: Mutex<BloubEngine>,
    renderer: Mutex<Option<BloubRenderer>>,
    flat: BloubFlatRenderer,
    render_active: AtomicBool,
    cpu: AtomicBool,
    target: Mutex<PetRenderer>,
    preview: bool,
}

impl Pet3dInner {
    #[cfg(test)]
    fn new() -> Self {
        Self::with_mode(false, false)
    }

    #[cfg(test)]
    fn new_preview() -> Self {
        Self::with_mode(true, false)
    }

    fn new_cpu() -> Self {
        Self::with_mode(false, true)
    }

    fn new_cpu_preview() -> Self {
        Self::with_mode(true, true)
    }

    fn with_mode(preview: bool, cpu: bool) -> Self {
        let now = Instant::now();
        Self {
            start: now,
            input: Mutex::new(Pet3dInput::default()),
            shared: Mutex::new(Pet3dShared {
                frame: None,
                palette: BloubPalette::LIGHT,
                dirty: true,
                ready: false,
                failed: false,
            }),
            timing: Mutex::new(Pet3dTiming {
                last_gaze: now,
                last_happy_generation: 0,
                last_dizzy_generation: 0,
                last_welcome_generation: 1,
                last_notification_generation: 0,
                notification_start: None,
                welcome_start: Some(now),
            }),
            engine: Mutex::new(BloubEngine::new()),
            renderer: Mutex::new(None),
            flat: BloubFlatRenderer::new(),
            render_active: AtomicBool::new(true),
            cpu: AtomicBool::new(cpu),
            target: Mutex::new(if cpu {
                PetRenderer::TwoD
            } else {
                PetRenderer::ThreeD
            }),
            preview,
        }
    }

    pub fn set_render_active(&self, active: bool) {
        self.render_active.store(active, Ordering::Relaxed);
    }

    pub fn is_render_active(&self) -> bool {
        self.render_active.load(Ordering::Relaxed)
    }

    pub fn is_cpu(&self) -> bool {
        self.cpu.load(Ordering::Relaxed)
    }

    pub fn cpu_image(&self) -> Option<slint::Image> {
        if !self.is_cpu() {
            return None;
        }
        let (frame, palette) = {
            let mut shared = self.shared.lock().unwrap();
            shared.dirty = false;
            (shared.frame.clone()?, shared.palette)
        };
        let size = self.flat.size();
        let rgba = self.flat.render(&frame, palette);
        let buffer =
            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&rgba, size, size);
        Some(slint::Image::from_rgba8(buffer))
    }

    pub fn gpu_image(&self) -> Option<slint::Image> {
        if self.is_cpu() {
            return None;
        }
        let (frame, palette) = {
            let mut shared = self.shared.lock().unwrap();
            shared.dirty = false;
            (shared.frame.clone()?, shared.palette)
        };
        let mut failure = None;
        let rendered = {
            let mut guard = self.renderer.lock().unwrap();
            let renderer = guard.as_mut()?;
            renderer.update(&frame, palette);
            renderer.render();
            match renderer.read_rgba() {
                Ok(rgba) => {
                    let size = renderer.image_size();
                    let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                        &rgba, size, size,
                    );
                    Some(slint::Image::from_rgba8(buffer))
                }
                Err(err) => {
                    failure = Some(err);
                    None
                }
            }
        };
        if let Some(err) = failure {
            warn!("Bloub 3D readback failed ({err}); switching to flat renderer");
            self.shared.lock().unwrap().failed = true;
            return self.use_cpu();
        }
        rendered
    }

    pub fn target(&self) -> PetRenderer {
        *self.target.lock().unwrap()
    }

    pub fn switch_renderer(&self, target: PetRenderer) -> Option<slint::Image> {
        *self.target.lock().unwrap() = target;
        if target.is_3d() {
            if !self.is_cpu() && self.renderer.lock().unwrap().is_some() {
                return self.gpu_image();
            }
            match create_gpu_renderer() {
                Ok(renderer) => {
                    *self.renderer.lock().unwrap() = Some(renderer);
                    self.cpu.store(false, Ordering::Relaxed);
                    {
                        let mut shared = self.shared.lock().unwrap();
                        shared.ready = true;
                        shared.failed = false;
                    }
                    info!("Bloub 3D renderer ready");
                    self.gpu_image().or_else(|| self.use_cpu())
                }
                Err(err) => {
                    warn!("Could not initialize Bloub 3D renderer ({err}); using flat renderer");
                    self.use_cpu()
                }
            }
        } else {
            self.use_cpu()
        }
    }

    fn use_cpu(&self) -> Option<slint::Image> {
        *self.renderer.lock().unwrap() = None;
        self.cpu.store(true, Ordering::Relaxed);
        self.shared.lock().unwrap().ready = true;
        self.cpu_image()
    }

    pub fn update_from_state(&self, snapshot: &ViewSnapshot) {
        let mut input = self.input.lock().unwrap();
        let preview = self.preview;
        input.is_singing = !preview
            && snapshot.domain.media.is_playing
            && snapshot.settings.behavior.music_reactions;
        input.bass = if preview {
            0.0
        } else {
            ((snapshot.domain.spectrum[0] + snapshot.domain.spectrum[1]) * 0.5).clamp(0.0, 1.0)
        };
        input.pet_is_happy = snapshot.presentation.pet_is_happy;
        input.pet_happy_generation = snapshot.presentation.pet_happy_generation;
        input.pet_dizzy_generation = snapshot.presentation.pet_dizzy_generation;
        input.is_welcome = !preview && snapshot.presentation.is_welcome;
        input.welcome_generation = snapshot.presentation.welcome_generation;
        input.is_dizzy = snapshot.presentation.pet_is_dizzy;
        input.is_sleepy = !preview
            && snapshot.presentation.pet_is_sleepy
            && snapshot.settings.behavior.sleepy_on_low_battery;
        input.is_charging = !preview && snapshot.domain.battery.is_charging;
        input.alert_kind = if preview {
            AlertKind::None
        } else {
            snapshot.domain.alert_kind
        };
        input.notification_generation = if preview {
            0
        } else {
            snapshot.presentation.notification_generation
        };
        input.is_compact =
            !preview && matches!(snapshot.navigation.current_view, IslandView::Compact);
        input.mochi_mood = if preview {
            MochiMoodState::Idle
        } else {
            snapshot.presentation.mochi_mood
        };
        input.blob_character = snapshot.settings.blob_character;
        input.blob_palette = snapshot.settings.blob_palette;
        input.blob_expression = snapshot.settings.blob_expression;
        input.blob_expression_custom = snapshot.settings.blob_expression_custom;
        input.cursor_gaze_enabled = !preview && snapshot.settings.behavior.cursor_gaze;
        input.idle_acting_enabled = snapshot.settings.behavior.idle_acting;
        input.notification_reactions_enabled =
            !preview && snapshot.settings.behavior.notification_reactions;
        input.welcome_age = if preview { -1.0 } else { 0.0 };

        let mut timing = self.timing.lock().unwrap();
        if preview {
            timing.welcome_start = None;
        } else if snapshot.presentation.welcome_generation != timing.last_welcome_generation {
            timing.welcome_start = if snapshot.presentation.is_welcome {
                Some(Instant::now())
            } else {
                None
            };
        }
        if !preview && !snapshot.presentation.is_welcome {
            timing.welcome_start = None;
        }
    }

    pub fn set_gaze(&self, x: f32, y: f32) {
        let mut input = self.input.lock().unwrap();
        if !input.cursor_gaze_enabled {
            input.gaze_active = false;
            return;
        }
        input.gaze_x = x.clamp(-1.0, 1.0);
        input.gaze_y = y.clamp(-1.0, 1.0);
        input.gaze_active = true;
        let mut timing = self.timing.lock().unwrap();
        timing.last_gaze = Instant::now();
    }

    pub fn tick(&self) -> TickResult {
        if !self.is_render_active() {
            return TickResult { dirty: false };
        }

        let now = Instant::now();
        let mut input = *self.input.lock().unwrap();
        let mut timing = self.timing.lock().unwrap();

        if !input.cursor_gaze_enabled
            || (input.gaze_active && now.duration_since(timing.last_gaze) > GAZE_TIMEOUT)
        {
            input.gaze_active = false;
        }

        if !input.notification_reactions_enabled {
            timing.notification_start = None;
            timing.last_notification_generation = input.notification_generation;
        }
        if input.notification_generation != timing.last_notification_generation {
            debug!(
                generation = input.notification_generation,
                "Bloub notification reaction started"
            );
            timing.notification_start = Some(now);
        }
        input.notification_active = input.notification_reactions_enabled
            && timing
                .notification_start
                .is_some_and(|start| now.duration_since(start) < Duration::from_secs_f32(2.4));

        if input.is_welcome {
            if let Some(start) = timing.welcome_start {
                input.welcome_age = now.duration_since(start).as_secs_f32();
            } else {
                timing.welcome_start = Some(now);
                input.welcome_age = 0.0;
            }
        } else {
            input.welcome_age = -1.0;
        }

        let target_state = resolve_state(&input);
        let expression = Some(resolve_expression(&input, target_state));

        let happy_restart = input.pet_happy_generation != timing.last_happy_generation;
        let dizzy_restart = input.pet_dizzy_generation != timing.last_dizzy_generation;
        let welcome_restart = input.welcome_generation != timing.last_welcome_generation;
        let notification_restart = input.notification_active
            && input.notification_generation != timing.last_notification_generation;
        timing.last_happy_generation = input.pet_happy_generation;
        timing.last_dizzy_generation = input.pet_dizzy_generation;
        timing.last_welcome_generation = input.welcome_generation;
        timing.last_notification_generation = input.notification_generation;

        let t = self.start.elapsed().as_secs_f32();
        let shape = ShapeId::from_index(input.blob_character as usize);
        let palette = BloubPalette::from_index(input.blob_palette as usize);

        let mut engine = self.engine.lock().unwrap();
        if (happy_restart
            && matches!(
                target_state,
                StateId::Wink
                    | StateId::Bounce
                    | StateId::Backflip
                    | StateId::RotateX
                    | StateId::Alert
            ))
            || (dizzy_restart
                && matches!(
                    target_state,
                    StateId::Orbit | StateId::Shake | StateId::RotateX
                ))
            || (welcome_restart && target_state == StateId::Swirl)
            || (notification_restart && target_state == StateId::Ping)
        {
            engine.restart_state(target_state, t);
        } else {
            engine.set_state(target_state, t);
        }
        match expression {
            Some(expression) => engine.set_expression(expression, t),
            None => engine.clear_expression(),
        }
        engine.set_shape(shape, t);
        if input.gaze_active {
            engine.set_look(input.gaze_x, input.gaze_y, true);
        } else {
            engine.set_look(0.0, 0.0, false);
        }
        engine.set_energy(input.bass);
        engine.set_compact(input.is_compact);
        engine.set_idle_acting(
            target_state == StateId::Idle
                && !input.blob_expression_custom
                && input.idle_acting_enabled,
        );
        let frame = engine.sample(t);
        drop(engine);

        let mut shared = self.shared.lock().unwrap();
        shared.frame = Some(frame);
        shared.palette = palette;
        shared.dirty = true;

        TickResult { dirty: true }
    }
}

fn create_gpu_renderer() -> Result<BloubRenderer, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("Could not create WGPU init runtime: {err}"))?;
    runtime.block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .map_err(|err| format!("No WGPU adapter: {err}"))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("bloub-standalone"),
                ..Default::default()
            })
            .await
            .map_err(|err| format!("No WGPU device: {err}"))?;
        BloubRenderer::new(device, queue)
    })
}

fn publish(window: &MainWindow, inner: &Pet3dInner) {
    let image = if inner.is_cpu() {
        inner.cpu_image()
    } else {
        inner.gpu_image()
    };
    let ctx = window.global::<IslandContext>();
    if let Some(image) = image {
        ctx.set_pet_3d_image(image);
        ctx.set_pet_3d_ready(true);
    }
}

pub fn install(window: &MainWindow) -> Arc<Pet3dInner> {
    let inner = Arc::new(Pet3dInner::new_cpu());
    inner.tick();
    publish(window, &inner);
    info!("Bloub 2D renderer ready");
    inner
}

pub fn install_settings(window: &SettingsWindow) -> Arc<Pet3dInner> {
    let inner = Arc::new(Pet3dInner::new_cpu_preview());
    inner.tick();
    let ctx = window.global::<IslandContext>();
    if let Some(image) = inner.cpu_image() {
        ctx.set_pet_3d_image(image);
        ctx.set_pet_3d_ready(true);
    }
    inner
}

pub fn start_timer(
    inner: Arc<Pet3dInner>,
    window: slint::Weak<MainWindow>,
    stats: Arc<RenderStats>,
) -> Rc<slint::Timer> {
    let timer = Rc::new(slint::Timer::default());
    timer.start(slint::TimerMode::Repeated, FRAME_INTERVAL, move || {
        if !inner.is_render_active() {
            return;
        }
        stats.pet_tick();
        inner.tick();
        if inner.is_cpu() {
            if let Some(image) = inner.cpu_image()
                && let Some(win) = window.upgrade()
            {
                let ctx = win.global::<IslandContext>();
                ctx.set_pet_3d_image(image);
                ctx.set_pet_3d_ready(true);
                stats.pet_redraw();
                win.window().request_redraw();
            }
        } else if let Some(image) = inner.gpu_image()
            && let Some(win) = window.upgrade()
        {
            let ctx = win.global::<IslandContext>();
            ctx.set_pet_3d_image(image);
            ctx.set_pet_3d_ready(true);
            stats.pet_redraw();
            win.window().request_redraw();
        }
    });
    timer
}

pub fn start_settings_timer(
    inner: Arc<Pet3dInner>,
    window: slint::Weak<SettingsWindow>,
) -> Rc<slint::Timer> {
    let timer = Rc::new(slint::Timer::default());
    timer.start(slint::TimerMode::Repeated, FRAME_INTERVAL, move || {
        let Some(win) = window.upgrade() else {
            return;
        };
        if !win.window().is_visible() {
            return;
        }
        inner.tick();
        if inner.is_cpu() {
            if let Some(image) = inner.cpu_image() {
                let ctx = win.global::<IslandContext>();
                ctx.set_pet_3d_image(image);
                ctx.set_pet_3d_ready(true);
                win.window().request_redraw();
            }
        } else if let Some(image) = inner.gpu_image() {
            let ctx = win.global::<IslandContext>();
            ctx.set_pet_3d_image(image);
            ctx.set_pet_3d_ready(true);
            win.window().request_redraw();
        }
    });
    timer
}
#[cfg(test)]
mod tests {
    use super::*;
    use island_core::state::AppState;

    fn idle_input() -> Pet3dInput {
        Pet3dInput {
            is_welcome: false,
            welcome_age: -1.0,
            ..Pet3dInput::default()
        }
    }

    fn input_with_expression(expression: u32, custom: bool) -> Pet3dInput {
        Pet3dInput {
            blob_expression: expression,
            blob_expression_custom: custom,
            ..idle_input()
        }
    }

    #[test]
    fn custom_expression_overrides_charging_mood() {
        let mut input = input_with_expression(ExpressionId::Robot.index() as u32, true);
        input.is_charging = true;
        assert_eq!(
            resolve_expression(&input, StateId::Charging),
            ExpressionId::Robot
        );
    }

    #[test]
    fn charging_still_uses_excite_without_custom_choice() {
        let mut input = input_with_expression(ExpressionId::Robot.index() as u32, false);
        input.is_charging = true;
        assert_eq!(
            resolve_expression(&input, StateId::Charging),
            ExpressionId::Excite
        );
    }

    #[test]
    fn charger_feedback_takes_priority_over_music() {
        let mut input = idle_input();
        input.is_singing = true;
        input.is_charging = true;
        assert_eq!(resolve_state(&input), StateId::Charging);

        input.alert_kind = AlertKind::Charging;
        assert_eq!(resolve_state(&input), StateId::Charge);

        input.notification_active = true;
        assert_eq!(resolve_state(&input), StateId::Ping);
    }

    #[test]
    fn pet_clicks_cycle_through_interactive_stunts() {
        let mut input = idle_input();
        input.pet_is_happy = true;
        let expected = [
            StateId::Wink,
            StateId::Bounce,
            StateId::Backflip,
            StateId::RotateX,
        ];
        for (generation, state) in expected.into_iter().enumerate() {
            input.pet_happy_generation = generation as u64;
            assert_eq!(resolve_state(&input), state);
        }
    }

    #[test]
    fn hidden_does_not_tick() {
        let inner = Pet3dInner::new();
        inner.set_render_active(false);

        let result = inner.tick();

        assert!(!result.dirty);
        assert!(inner.shared.lock().unwrap().frame.is_none());
    }

    #[test]
    fn visible_tick_is_dirty() {
        let inner = Pet3dInner::new();

        let result = inner.tick();

        assert!(result.dirty);
        assert!(inner.shared.lock().unwrap().frame.is_some());
    }

    #[test]
    fn cpu_renderer_publishes_image() {
        let inner = Pet3dInner::new_cpu();
        inner.tick();

        let image = inner.cpu_image().expect("cpu image");

        assert_eq!(image.size().width, 192);
        assert_eq!(image.size().height, 192);
    }

    #[test]
    fn reactivation_ticks_after_inactive_gap() {
        let inner = Pet3dInner::new();
        inner.set_render_active(false);
        assert!(!inner.tick().dirty);

        inner.set_render_active(true);

        assert!(inner.tick().dirty);
    }

    #[test]
    fn preview_ignores_cursor_gaze() {
        let inner = Pet3dInner::new_preview();
        let mut state = AppState::default();
        state.settings.behavior.cursor_gaze = true;
        inner.update_from_state(&state.snapshot());

        inner.set_gaze(0.75, -0.35);
        assert!(!inner.input.lock().unwrap().gaze_active);
    }

    #[test]
    fn preview_ignores_welcome_media_and_alerts() {
        let inner = Pet3dInner::new_preview();
        let mut state = AppState::default();
        state.domain.media.is_playing = true;
        state.presentation.is_welcome = true;
        state.domain.battery.is_charging = true;
        state.domain.alert_kind = AlertKind::Charging;
        state.settings.blob_character = 4;
        state.settings.blob_palette = 9;

        inner.update_from_state(&state.snapshot());
        inner.tick();

        let input = *inner.input.lock().unwrap();
        assert!(!input.is_singing);
        assert!(!input.is_welcome);
        assert!(!input.is_charging);
        assert_eq!(input.alert_kind, AlertKind::None);
        assert_eq!(input.blob_character, 4);
        assert_eq!(input.blob_palette, 9);

        let shared = inner.shared.lock().unwrap();
        assert!(shared.frame.is_some());
    }

    #[test]
    fn disabled_gaze_ignores_cursor_updates() {
        let inner = Pet3dInner::new();
        inner.input.lock().unwrap().cursor_gaze_enabled = false;
        inner.set_gaze(0.8, -0.4);
        assert!(!inner.input.lock().unwrap().gaze_active);
    }

    #[test]
    fn disabled_behavior_gates_pet_inputs() {
        let mut state = AppState::default();
        state.domain.media.is_playing = true;
        state.presentation.pet_is_sleepy = true;
        state.presentation.notification_generation = 1;
        state.settings.behavior.music_reactions = false;
        state.settings.behavior.sleepy_on_low_battery = false;
        state.settings.behavior.notification_reactions = false;

        let inner = Pet3dInner::new();
        inner.update_from_state(&state.snapshot());
        {
            let input = inner.input.lock().unwrap();
            assert!(!input.is_singing);
            assert!(!input.is_sleepy);
            assert!(!input.notification_reactions_enabled);
        }

        inner.tick();
        assert!(!inner.input.lock().unwrap().notification_active);
    }

    #[test]
    fn alerts_music_and_dizzy_map_to_animated_states() {
        let mut input = idle_input();
        input.alert_kind = AlertKind::LowBattery;
        assert_eq!(resolve_state(&input), StateId::Alert);

        input.alert_kind = AlertKind::Charging;
        assert_eq!(resolve_state(&input), StateId::Charge);

        input.alert_kind = AlertKind::None;
        input.is_charging = true;
        assert_eq!(resolve_state(&input), StateId::Charging);
        input.is_charging = false;
        input.notification_active = true;
        assert_eq!(resolve_state(&input), StateId::Ping);
        input.is_charging = true;
        assert_eq!(resolve_state(&input), StateId::Ping);
        input.is_charging = false;
        input.is_sleepy = true;
        assert_eq!(resolve_state(&input), StateId::Ping);
        input.is_sleepy = false;
        input.notification_active = false;

        input.is_singing = true;
        assert_eq!(resolve_state(&input), StateId::Orbit);

        input.is_singing = false;
        input.is_dizzy = true;
        input.pet_dizzy_generation = 0;
        assert_eq!(resolve_state(&input), StateId::Orbit);
        input.pet_dizzy_generation = 1;
        assert_eq!(resolve_state(&input), StateId::Shake);
        input.pet_dizzy_generation = 2;
        assert_eq!(resolve_state(&input), StateId::RotateX);
    }
}
