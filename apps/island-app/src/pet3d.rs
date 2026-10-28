use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use island_core::state::AppState;
use island_pet3d::{Pet3dRenderer, Pet3dState};
use island_ui::{IslandContext, MainWindow};
use slint::{ComponentHandle, GraphicsAPI, RenderingState};
use tracing::{info, warn};

const ACTIVE_INTERVAL: Duration = Duration::from_millis(33);
const IDLE_INTERVAL: Duration = Duration::from_millis(250);
const BLINK_DURATION: Duration = Duration::from_millis(180);
const HAPPY_DURATION: f32 = 1.5;
const WELCOME_DURATION: f32 = 3.5;

#[derive(Clone, Copy, Debug)]
struct Pet3dInput {
    is_singing: bool,
    bass: f32,
    pet_is_happy: bool,
    pet_happy_generation: u64,
    is_welcome: bool,
    welcome_generation: u64,
    gaze_x: f32,
    gaze_y: f32,
}

impl Default for Pet3dInput {
    fn default() -> Self {
        Self {
            is_singing: false,
            bass: 0.0,
            pet_is_happy: false,
            pet_happy_generation: 0,
            is_welcome: true,
            welcome_generation: 1,
            gaze_x: 0.0,
            gaze_y: 0.0,
        }
    }
}

struct Pet3dShared {
    state: Pet3dState,
    dirty: bool,
    ready: bool,
    failed: bool,
}

struct Pet3dTiming {
    last_happy_generation: u64,
    happy_start: Option<Instant>,
    last_welcome_generation: u64,
    welcome_start: Option<Instant>,
    blink_start: Option<Instant>,
    next_blink: Instant,
    rng: u64,
    last_active: bool,
}

pub struct TickResult {
    pub dirty: bool,
    pub active: bool,
}

pub struct Pet3dInner {
    start: Instant,
    input: Mutex<Pet3dInput>,
    shared: Mutex<Pet3dShared>,
    timing: Mutex<Pet3dTiming>,
    renderer: Mutex<Option<Pet3dRenderer>>,
}

impl Pet3dInner {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            input: Mutex::new(Pet3dInput::default()),
            shared: Mutex::new(Pet3dShared {
                state: Pet3dState::default(),
                dirty: true,
                ready: false,
                failed: false,
            }),
            timing: Mutex::new(Pet3dTiming {
                last_happy_generation: 0,
                happy_start: None,
                last_welcome_generation: 1,
                welcome_start: Some(now),
                blink_start: None,
                next_blink: now + Duration::from_secs(2),
                rng: 0x5eed_1234_abcd_9876,
                last_active: false,
            }),
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
        input.is_welcome = app_state.is_welcome;
        input.welcome_generation = app_state.welcome_generation;
    }

    pub fn set_gaze(&self, x: f32, y: f32) {
        let mut input = self.input.lock().unwrap();
        input.gaze_x = x.clamp(-1.0, 1.0);
        input.gaze_y = y.clamp(-1.0, 1.0);
    }

    pub fn tick(&self) -> TickResult {
        let now = Instant::now();
        let input = *self.input.lock().unwrap();
        let mut timing = self.timing.lock().unwrap();

        if input.pet_happy_generation != timing.last_happy_generation {
            timing.last_happy_generation = input.pet_happy_generation;
            timing.happy_start = if input.pet_is_happy { Some(now) } else { None };
        }
        if !input.pet_is_happy {
            timing.happy_start = None;
        } else if timing.happy_start.is_none() {
            timing.happy_start = Some(now);
        }

        if input.welcome_generation != timing.last_welcome_generation {
            timing.last_welcome_generation = input.welcome_generation;
            timing.welcome_start = if input.is_welcome { Some(now) } else { None };
        }
        if !input.is_welcome {
            timing.welcome_start = None;
        } else if timing.welcome_start.is_none() {
            timing.welcome_start = Some(now);
        }

        if timing
            .blink_start
            .is_some_and(|start| now.duration_since(start) >= BLINK_DURATION)
        {
            timing.blink_start = None;
        }
        if timing.blink_start.is_none() && now >= timing.next_blink {
            timing.blink_start = Some(now);
            timing.rng = timing
                .rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let delay_ms = 2_200 + ((timing.rng >> 33) % 2_800);
            timing.next_blink = now + Duration::from_millis(delay_ms);
        }

        let happy_age = timing
            .happy_start
            .map(|start| now.duration_since(start).as_secs_f32())
            .filter(|age| *age < HAPPY_DURATION)
            .unwrap_or(-1.0);
        let welcome_age = timing
            .welcome_start
            .map(|start| now.duration_since(start).as_secs_f32())
            .filter(|age| *age < WELCOME_DURATION)
            .unwrap_or(-1.0);
        let blink_age = timing
            .blink_start
            .map(|start| now.duration_since(start).as_secs_f32())
            .unwrap_or(-1.0);

        let state = Pet3dState {
            is_singing: input.is_singing,
            bass: input.bass,
            happy_age,
            welcome_age,
            blink_age,
            gaze_x: input.gaze_x,
            gaze_y: input.gaze_y,
        };
        let active = state.is_singing
            || state.happy_age >= 0.0
            || state.welcome_age >= 0.0
            || state.blink_age >= 0.0;

        let mut shared = self.shared.lock().unwrap();
        let changed = state != shared.state || active != timing.last_active;
        timing.last_active = active;
        shared.state = state;
        shared.dirty |= changed;

        TickResult {
            dirty: shared.dirty,
            active,
        }
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
                        warn!("Momo 3D renderer requires the WGPU backend; using sprites");
                        self.shared.lock().unwrap().failed = true;
                        return;
                    }
                };

                match Pet3dRenderer::new(device, queue) {
                    Ok(mut renderer) => {
                        let pet_state = self.shared.lock().unwrap().state;
                        renderer.update(pet_state, self.start.elapsed().as_secs_f32());
                        renderer.render();

                        let image = match renderer.image() {
                            Ok(image) => image,
                            Err(err) => {
                                warn!("Could not import Momo texture into Slint: {err}");
                                self.shared.lock().unwrap().failed = true;
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
                        info!("Momo 3D renderer ready");
                    }
                    Err(err) => {
                        warn!("Could not initialize Momo 3D renderer: {err}");
                        self.shared.lock().unwrap().failed = true;
                        if let Some(win) = window.upgrade() {
                            win.global::<IslandContext>().set_pet_3d_ready(false);
                        }
                    }
                }
            }
            RenderingState::BeforeRendering => {
                let pet_state = {
                    let mut shared = self.shared.lock().unwrap();
                    if !shared.ready || !shared.dirty {
                        return;
                    }
                    shared.dirty = false;
                    shared.state
                };

                if let Some(renderer) = self.renderer.lock().unwrap().as_mut() {
                    renderer.update(pet_state, self.start.elapsed().as_secs_f32());
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
        warn!("Could not install Momo rendering notifier: {err}");
    }

    inner
}

pub fn start_timer(
    inner: Arc<Pet3dInner>,
    window: slint::Weak<MainWindow>,
) -> Rc<slint::Timer> {
    let timer = Rc::new(slint::Timer::default());
    let timer_cb = timer.clone();
    timer.start(
        slint::TimerMode::Repeated,
        IDLE_INTERVAL,
        move || {
            let result = inner.tick();
            timer_cb.set_interval(if result.active {
                ACTIVE_INTERVAL
            } else {
                IDLE_INTERVAL
            });
            if result.dirty
                && let Some(win) = window.upgrade()
            {
                win.window().request_redraw();
            }
        },
    );
    timer
}
