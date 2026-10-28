use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use island_core::models::{AlertKind, IslandView};
use island_core::state::{AppState, MochiMoodState};
use island_pet3d::{
    BloubEngine, BloubFrame, BloubPalette, BloubRenderer, ExpressionId, ShapeId, StateId,
};
use island_ui::{IslandContext, MainWindow};
use slint::{ComponentHandle, GraphicsAPI, RenderingState};
use tracing::{info, warn};

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
        }
    }
}

fn resolve_state(input: &Pet3dInput) -> StateId {
    if input.is_dizzy {
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
    } else if input.mochi_mood == MochiMoodState::Annoyed {
        StateId::Alert
    } else if input.notification_active {
        StateId::Notify
    } else if input.is_singing {
        StateId::Orbit
    } else if input.alert_kind == AlertKind::LowBattery {
        StateId::Alert
    } else if input.is_charging || input.alert_kind != AlertKind::None {
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
    } else if target_state == StateId::Notify {
        ExpressionId::Excite
    } else if input.is_sleepy {
        ExpressionId::Somnolent
    } else if input.mochi_mood == MochiMoodState::Annoyed {
        ExpressionId::Colere
    } else if input.pet_is_happy {
        ExpressionId::Heureux
    } else if input.is_singing {
        ExpressionId::Hilare
    } else if input.is_charging {
        ExpressionId::Excite
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
}

impl Pet3dInner {
    fn new() -> Self {
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
        }
    }

    pub fn update_from_state(&self, app_state: &AppState) {
        let mut input = self.input.lock().unwrap();
        input.is_singing = app_state.media.is_playing;
        input.bass =
            ((app_state.spectrum.bars[0] + app_state.spectrum.bars[1]) * 0.5).clamp(0.0, 1.0);
        input.pet_is_happy = app_state.pet_is_happy;
        input.pet_happy_generation = app_state.pet_happy_generation;
        input.pet_dizzy_generation = app_state.pet_dizzy_generation;
        input.is_welcome = app_state.is_welcome;
        input.welcome_generation = app_state.welcome_generation;
        input.is_dizzy = app_state.pet_is_dizzy;
        input.is_sleepy = app_state.pet_is_sleepy;
        input.is_charging = app_state.battery.is_charging;
        input.alert_kind = app_state.alert_kind;
        input.notification_generation = app_state.notification_generation;
        input.is_compact = matches!(app_state.mode, IslandView::Compact);
        input.mochi_mood = app_state.mochi_mood;
        input.blob_character = app_state.blob_character;
        input.blob_palette = app_state.blob_palette;
        input.blob_expression = app_state.blob_expression;
        input.blob_expression_custom = app_state.blob_expression_custom;
        input.welcome_age = 0.0;

        let mut timing = self.timing.lock().unwrap();
        if app_state.welcome_generation != timing.last_welcome_generation {
            timing.welcome_start = if app_state.is_welcome {
                Some(Instant::now())
            } else {
                None
            };
        }
        if !app_state.is_welcome {
            timing.welcome_start = None;
        }
    }

    pub fn set_gaze(&self, x: f32, y: f32) {
        let mut input = self.input.lock().unwrap();
        input.gaze_x = x.clamp(-1.0, 1.0);
        input.gaze_y = y.clamp(-1.0, 1.0);
        input.gaze_active = true;
        let mut timing = self.timing.lock().unwrap();
        timing.last_gaze = Instant::now();
    }

    pub fn tick(&self) -> TickResult {
        let now = Instant::now();
        let mut input = *self.input.lock().unwrap();
        let mut timing = self.timing.lock().unwrap();

        if input.gaze_active && now.duration_since(timing.last_gaze) > GAZE_TIMEOUT {
            input.gaze_active = false;
        }

        if input.notification_generation != timing.last_notification_generation {
            timing.notification_start = Some(now);
        }
        input.notification_active = timing
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
                StateId::Wink | StateId::Bounce | StateId::Backflip | StateId::RotateX | StateId::Alert
            ))
            || (dizzy_restart && matches!(target_state, StateId::Orbit | StateId::Shake | StateId::RotateX))
            || (welcome_restart && target_state == StateId::Swirl)
            || (notification_restart && target_state == StateId::Notify)
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
        engine.set_idle_acting(target_state == StateId::Idle && !input.blob_expression_custom);
        let frame = engine.sample(t);
        drop(engine);

        let mut shared = self.shared.lock().unwrap();
        shared.frame = Some(frame);
        shared.palette = palette;
        shared.dirty = true;

        TickResult { dirty: true }
    }

    fn notify(
        &self,
        state: RenderingState,
        graphics_api: &GraphicsAPI<'_>,
        window: &slint::Weak<MainWindow>,
    ) {
        match state {
            RenderingState::RenderingSetup => {
                let (device, queue) = match graphics_api {
                    GraphicsAPI::WGPU30 { device, queue, .. } => {
                        (device.clone(), queue.clone())
                    }
                    _ => {
                        warn!("Bloub 3D renderer requires the WGPU backend; using vector fallback");
                        self.shared.lock().unwrap().failed = true;
                        if let Some(win) = window.upgrade() {
                            win.global::<IslandContext>().set_pet_3d_ready(false);
                        }
                        return;
                    }
                };

                match BloubRenderer::new(device, queue) {
                    Ok(mut renderer) => {
                        let (frame, palette) = {
                            let shared = self.shared.lock().unwrap();
                            (shared.frame.clone(), shared.palette)
                        };
                        if let Some(frame) = frame {
                            renderer.update(&frame, palette);
                        }
                        renderer.render();

                        let image = match renderer.image() {
                            Ok(image) => image,
                            Err(err) => {
                                warn!("Could not import Bloub texture into Slint: {err}");
                                self.shared.lock().unwrap().failed = true;
                                if let Some(win) = window.upgrade() {
                                    win.global::<IslandContext>().set_pet_3d_ready(false);
                                }
                                return;
                            }
                        };

                        if let Some(win) = window.upgrade() {
                            let ctx = win.global::<IslandContext>();
                            ctx.set_pet_3d_image(image);
                            ctx.set_pet_3d_ready(true);
                        }

                        *self.renderer.lock().unwrap() = Some(renderer);
                        let mut shared = self.shared.lock().unwrap();
                        shared.ready = true;
                        shared.failed = false;
                        shared.dirty = false;
                        info!("Bloub 3D renderer ready");
                    }
                    Err(err) => {
                        warn!("Could not initialize Bloub 3D renderer: {err}");
                        self.shared.lock().unwrap().failed = true;
                        if let Some(win) = window.upgrade() {
                            win.global::<IslandContext>().set_pet_3d_ready(false);
                        }
                    }
                }
            }
            RenderingState::BeforeRendering => {
                let (frame, palette) = {
                    let mut shared = self.shared.lock().unwrap();
                    if !shared.ready || !shared.dirty {
                        return;
                    }
                    shared.dirty = false;
                    (shared.frame.clone(), shared.palette)
                };

                if let Some(frame) = frame
                    && let Some(renderer) = self.renderer.lock().unwrap().as_mut()
                {
                    renderer.update(&frame, palette);
                    renderer.render();
                }
            }
            RenderingState::RenderingTeardown => {
                *self.renderer.lock().unwrap() = None;
                let mut shared = self.shared.lock().unwrap();
                shared.ready = false;
                shared.dirty = false;
            }
            _ => {}
        }
    }
}

pub fn install(window: &MainWindow) -> Arc<Pet3dInner> {
    let inner = Arc::new(Pet3dInner::new());
    inner.tick();

    let weak = window.as_weak();
    let inner_cb = inner.clone();
    if let Err(err) = window
        .window()
        .set_rendering_notifier(move |state, graphics_api| {
            inner_cb.notify(state, graphics_api, &weak);
        })
    {
        warn!("Could not install Bloub rendering notifier: {err}");
    }

    inner
}

pub fn start_timer(
    inner: Arc<Pet3dInner>,
    window: slint::Weak<MainWindow>,
) -> Rc<slint::Timer> {
    let timer = Rc::new(slint::Timer::default());
    timer.start(slint::TimerMode::Repeated, FRAME_INTERVAL, move || {
        let result = inner.tick();
        if result.dirty
            && let Some(win) = window.upgrade()
        {
            win.window().request_redraw();
        }
    });
    timer
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(resolve_expression(&input, StateId::Idle), ExpressionId::Robot);
    }

    #[test]
    fn charging_still_uses_excite_without_custom_choice() {
        let mut input = input_with_expression(ExpressionId::Robot.index() as u32, false);
        input.is_charging = true;
        assert_eq!(resolve_expression(&input, StateId::Idle), ExpressionId::Excite);
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
    fn alerts_music_and_dizzy_map_to_animated_states() {
        let mut input = idle_input();
        input.alert_kind = AlertKind::LowBattery;
        assert_eq!(resolve_state(&input), StateId::Alert);

        input.alert_kind = AlertKind::Charging;
        assert_eq!(resolve_state(&input), StateId::Notify);

        input.alert_kind = AlertKind::None;
        input.notification_active = true;
        assert_eq!(resolve_state(&input), StateId::Notify);
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
