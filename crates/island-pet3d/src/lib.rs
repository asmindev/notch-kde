//! Offscreen 3D renderer for the notchde pet avatar.

mod loader;
mod renderer;

pub use loader::{
    compute_worlds, load_momo, load_momo_from_slice, Material, MeshAsset, Node, NodeKind, Vertex,
};
pub use renderer::Pet3dRenderer;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pet3dState {
    pub is_singing: bool,
    pub bass: f32,
    pub happy_age: f32,
    pub welcome_age: f32,
    pub blink_age: f32,
    pub gaze_x: f32,
    pub gaze_y: f32,
}

impl Default for Pet3dState {
    fn default() -> Self {
        Self {
            is_singing: false,
            bass: 0.0,
            happy_age: -1.0,
            welcome_age: -1.0,
            blink_age: -1.0,
            gaze_x: 0.0,
            gaze_y: 0.0,
        }
    }
}
