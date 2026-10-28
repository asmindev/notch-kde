use island_core::models::{AlertKind, IslandView};
use island_core::AppState;
use island_platform::overlap::Rect;
use island_ui::MainWindow;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use slint::ComponentHandle;
use std::ffi::c_void;
use std::sync::mpsc;
use tracing::{debug, info, warn};
use wayland_client::backend::{Backend, ObjectId};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{
    wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry, wl_surface::WlSurface,
};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

const WINDOW_WIDTH: i32 = 480;
const COMPACT_WIDTH: i32 = 156;
const COMPACT_ALERT_WIDTH: i32 = 228;
const COMPACT_TOUCH_HEIGHT: i32 = 42;
const WIDE_WIDTH: i32 = 390;
const WIDE_TOUCH_HEIGHT: i32 = 98;
const EXPAND_WIDTH: i32 = 440;
const EXPAND_TOUCH_HEIGHT: i32 = 180;
const HIDDEN_TOUCH_HEIGHT: i32 = 28;
const COMPACT_EAR: i32 = 5;
const WIDE_EAR: i32 = 10;

pub fn rect_for_state(state: &AppState) -> Rect {
    let (notch_w, touch_h, ear) = if !state.is_visible {
        (COMPACT_WIDTH, HIDDEN_TOUCH_HEIGHT, COMPACT_EAR)
    } else {
        match state.mode {
            IslandView::Compact => {
                let width = if state.alert_kind != AlertKind::None {
                    COMPACT_ALERT_WIDTH
                } else {
                    COMPACT_WIDTH
                };
                (width, COMPACT_TOUCH_HEIGHT, COMPACT_EAR)
            }
            IslandView::Wide => (WIDE_WIDTH, WIDE_TOUCH_HEIGHT, WIDE_EAR),
            IslandView::Expand => (EXPAND_WIDTH, EXPAND_TOUCH_HEIGHT, WIDE_EAR),
        }
    };

    Rect {
        x: (WINDOW_WIDTH - notch_w) / 2 - ear,
        y: 0,
        w: (notch_w + ear * 2) as u32,
        h: touch_h as u32,
    }
}

pub fn wayland_handles(window: &slint::Window) -> Option<(usize, usize)> {
    let handle = window.window_handle();
    let display = handle.display_handle().ok()?.as_raw();
    let surface = handle.window_handle().ok()?.as_raw();
    match (display, surface) {
        (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(surface)) => Some((
            display.display.as_ptr() as usize,
            surface.surface.as_ptr() as usize,
        )),
        _ => None,
    }
}

pub fn schedule_init(
    window: slint::Weak<MainWindow>,
    region: InputRegionHandle,
    rect: Rect,
    attempts: u32,
) {
    slint::Timer::single_shot(std::time::Duration::from_millis(100), move || {
        let Some(window) = window.upgrade() else {
            return;
        };
        if let Some((display, surface)) = wayland_handles(window.window()) {
            region.init(display, surface, rect);
        } else if attempts > 0 {
            schedule_init(window.as_weak(), region, rect, attempts - 1);
        } else {
            warn!("Wayland window handle unavailable; input region was not applied");
        }
    });
}

#[derive(Clone)]
pub struct InputRegionHandle {
    tx: mpsc::Sender<Command>,
}

enum Command {
    Init {
        display: usize,
        surface: usize,
        rect: Rect,
    },
    SetRect(Rect),
}

impl InputRegionHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut pending = None;
            let mut region = None;

            while let Ok(command) = rx.recv() {
                match command {
                    Command::Init {
                        display,
                        surface,
                        rect,
                    } => {
                        let rect = pending.take().unwrap_or(rect);
                        match WaylandInputRegion::new(display, surface) {
                            Ok(mut new_region) => {
                                new_region.set_rect(rect);
                                region = Some(new_region);
                                info!("Applied Wayland input region from surface-local rect");
                            }
                            Err(err) => warn!("Failed to create Wayland input region: {err}"),
                        }
                    }
                    Command::SetRect(rect) => {
                        if let Some(region) = region.as_mut() {
                            region.set_rect(rect);
                        } else {
                            pending = Some(rect);
                        }
                    }
                }
            }
        });
        Self { tx }
    }

    pub fn init(&self, display: usize, surface: usize, rect: Rect) {
        let _ = self.tx.send(Command::Init {
            display,
            surface,
            rect,
        });
    }

    pub fn set_rect(&self, rect: Rect) {
        let _ = self.tx.send(Command::SetRect(rect));
    }
}

struct InputRegionState;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for InputRegionState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlCompositor, ()> for InputRegionState {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: <WlCompositor as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlRegion, ()> for InputRegionState {
    fn event(
        _: &mut Self,
        _: &WlRegion,
        _: <WlRegion as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

struct WaylandInputRegion {
    connection: Connection,
    compositor: WlCompositor,
    surface: WlSurface,
    queue_handle: QueueHandle<InputRegionState>,
    _event_queue: EventQueue<InputRegionState>,
}

impl WaylandInputRegion {
    fn new(display: usize, surface: usize) -> Result<Self, String> {
        let backend = unsafe {
            Backend::from_foreign_display(display as *mut c_void as *mut _)
        };
        let connection = Connection::from_backend(backend);
        let (globals, event_queue) = registry_queue_init::<InputRegionState>(&connection)
            .map_err(|err| format!("registry init failed: {err}"))?;
        let queue_handle = event_queue.handle();
        let compositor = globals
            .bind::<WlCompositor, _, _>(&queue_handle, 1..=6, ())
            .map_err(|err| format!("wl_compositor unavailable: {err}"))?;
        let object_id = unsafe {
            ObjectId::from_ptr(WlSurface::interface(), surface as *mut c_void as *mut _)
        }
        .map_err(|_| "invalid wl_surface pointer".to_string())?;
        let surface = WlSurface::from_id(&connection, object_id)
            .map_err(|_| "wl_surface is no longer alive".to_string())?;

        Ok(Self {
            connection,
            compositor,
            surface,
            queue_handle,
            _event_queue: event_queue,
        })
    }

    fn set_rect(&mut self, rect: Rect) {
        let region = self.compositor.create_region(&self.queue_handle, ());
        region.add(rect.x, rect.y, rect.w as i32, rect.h as i32);
        self.surface.set_input_region(Some(&region));
        region.destroy();
        if let Err(err) = self.connection.flush() {
            warn!("Failed to flush Wayland input region: {err}");
        } else {
            debug!(
                "Wayland input region set to ({},{} {}x{})",
                rect.x, rect.y, rect.w, rect.h
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(mode: IslandView, visible: bool, alert: AlertKind) -> AppState {
        let mut state = AppState::default();
        state.mode = mode;
        state.is_visible = visible;
        state.alert_kind = alert;
        state
    }

    #[test]
    fn compact_region_matches_visible_touch_area() {
        assert_eq!(
            rect_for_state(&state(IslandView::Compact, true, AlertKind::None)),
            Rect { x: 157, y: 0, w: 166, h: 42 }
        );
    }

    #[test]
    fn compact_alert_region_expands() {
        assert_eq!(
            rect_for_state(&state(IslandView::Compact, true, AlertKind::Charging)),
            Rect { x: 121, y: 0, w: 238, h: 42 }
        );
    }

    #[test]
    fn wide_region_matches_visible_touch_area() {
        assert_eq!(
            rect_for_state(&state(IslandView::Wide, true, AlertKind::None)),
            Rect { x: 35, y: 0, w: 410, h: 98 }
        );
    }

    #[test]
    fn expand_region_matches_visible_touch_area() {
        assert_eq!(
            rect_for_state(&state(IslandView::Expand, true, AlertKind::None)),
            Rect { x: 10, y: 0, w: 460, h: 180 }
        );
    }

    #[test]
    fn hidden_region_keeps_reveal_hit_target() {
        assert_eq!(
            rect_for_state(&state(IslandView::Wide, false, AlertKind::None)),
            Rect { x: 157, y: 0, w: 166, h: 28 }
        );
    }
}
