mod decor;
mod engine;
mod expressions;
mod face;
mod flat;
mod math;
mod profiles;
mod renderer;
mod shape;
mod skins;
mod states;

pub use engine::{BloubEngine, BloubFrame};
pub use expressions::ExpressionId;
pub use face::Look;
pub use flat::BloubFlatRenderer;
pub use renderer::BloubRenderer;
pub use skins::{ColorId, ShapeId, hex_to_rgba, luminance};
pub use states::StateId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BloubPalette {
    pub body: [f32; 4],
    pub eye: [f32; 4],
}

impl BloubPalette {
    pub const LIGHT: Self = Self {
        body: [0.929, 0.929, 0.937, 1.0],
        eye: [0.043, 0.043, 0.047, 1.0],
    };

    pub const DARK: Self = Self {
        body: [0.039, 0.039, 0.047, 1.0],
        eye: [0.945, 0.937, 0.914, 1.0],
    };

    pub const COUNT: usize = 14;

    pub fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => Self::LIGHT,
            1 => Self::DARK,
            n => {
                let color = ColorId::from_index(n - 2);
                let body = hex_to_rgba(color.hex());
                let eye = if luminance(body) > 0.48 {
                    Self::LIGHT.eye
                } else {
                    Self::DARK.eye
                };
                Self { body, eye }
            }
        }
    }

    pub fn label(index: usize) -> &'static str {
        match index % Self::COUNT {
            0 => "clair",
            1 => "encre",
            n => ColorId::from_index(n - 2).name(),
        }
    }

    pub fn swatch(index: usize) -> [f32; 4] {
        Self::from_index(index).body
    }
}

impl Default for BloubPalette {
    fn default() -> Self {
        Self::LIGHT
    }
}

#[cfg(test)]
mod tests;
