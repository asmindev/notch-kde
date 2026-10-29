use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

unsafe extern "C" {
    fn dodge_create(
        notch_x: i32,
        notch_y: i32,
        notch_w: u32,
        notch_h: u32,
        callback: extern "C" fn(bool, *mut c_void),
        user_data: *mut c_void,
    ) -> *mut c_void;

    fn dodge_run(ctx: *mut c_void);
    fn dodge_update_notch(ctx: *mut c_void, notch_x: i32, notch_y: i32, notch_w: u32, notch_h: u32);
    fn dodge_stop(ctx: *mut c_void);
    fn dodge_destroy(ctx: *mut c_void);
}

extern "C" fn on_overlap_callback(overlapping: bool, user_data: *mut c_void) {
    if user_data.is_null() {
        return;
    }
    let sender = unsafe { &*(user_data as *const UnboundedSender<bool>) };
    debug!("Native DodgeController overlap detected: {}", overlapping);
    let _ = sender.send(overlapping);
}

pub struct OverlapDetector {
    running: Arc<AtomicBool>,
    ctx_ptr: Arc<AtomicPtr<c_void>>,
}

impl OverlapDetector {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            ctx_ptr: Arc::new(AtomicPtr::new(std::ptr::null_mut())),
        }
    }

    pub fn start(&self, notch_rect: Rect, sender: UnboundedSender<bool>, enabled: bool) {
        if !enabled {
            info!("Dodge detection disabled by configuration");
            return;
        }

        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let running = Arc::clone(&self.running);
        let ctx_ptr_clone = Arc::clone(&self.ctx_ptr);

        std::thread::spawn(move || {
            let sender_box = Box::new(sender);
            let user_data = Box::into_raw(sender_box) as *mut c_void;

            info!(
                "Starting native C dodge controller at rect: ({},{} {}x{})",
                notch_rect.x, notch_rect.y, notch_rect.w, notch_rect.h
            );

            let ctx = unsafe {
                dodge_create(
                    notch_rect.x,
                    notch_rect.y,
                    notch_rect.w,
                    notch_rect.h,
                    on_overlap_callback,
                    user_data,
                )
            };

            if ctx.is_null() {
                error!("Failed to create native dodge context from Wayland display");
                unsafe { drop(Box::from_raw(user_data as *mut UnboundedSender<bool>)) };
                running.store(false, Ordering::SeqCst);
                return;
            }

            ctx_ptr_clone.store(ctx, Ordering::SeqCst);
            info!("Native DodgeController running on Wayland dispatch loop");
            unsafe {
                dodge_run(ctx);
                ctx_ptr_clone.store(std::ptr::null_mut(), Ordering::SeqCst);
                dodge_destroy(ctx);
                drop(Box::from_raw(user_data as *mut UnboundedSender<bool>));
            }
            info!("Native DodgeController stopped cleanly");
            running.store(false, Ordering::SeqCst);
        });
    }

    pub fn update_rect(&self, notch_rect: Rect) {
        let ptr = self.ctx_ptr.load(Ordering::SeqCst);
        if !ptr.is_null() {
            unsafe {
                dodge_update_notch(ptr, notch_rect.x, notch_rect.y, notch_rect.w, notch_rect.h);
            }
        }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        let ptr = self.ctx_ptr.swap(std::ptr::null_mut(), Ordering::SeqCst);
        if !ptr.is_null() {
            unsafe {
                dodge_stop(ptr);
            }
        }
    }
}

impl Drop for OverlapDetector {
    fn drop(&mut self) {
        self.stop();
    }
}
