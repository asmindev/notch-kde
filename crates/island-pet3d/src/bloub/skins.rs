use std::sync::OnceLock;

use super::math::TAU;
use super::profiles::PROFILE_SAMPLES;
use super::shape::{
    hull_of_circles, profile_from_polygon, regular_polygon_profile, superellipse_profile,
    union_of_circles_profile,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeId {
    Cercle,
    Galet,
    Squircle,
    Capsule,
    Triangle,
    Hexagone,
    Nuage,
    Goutte,
}

impl ShapeId {
    pub const ALL: [Self; 8] = [
        Self::Cercle,
        Self::Galet,
        Self::Squircle,
        Self::Capsule,
        Self::Triangle,
        Self::Hexagone,
        Self::Nuage,
        Self::Goutte,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Cercle => "cercle",
            Self::Galet => "galet",
            Self::Squircle => "squircle",
            Self::Capsule => "capsule",
            Self::Triangle => "triangle",
            Self::Hexagone => "hexagone",
            Self::Nuage => "nuage",
            Self::Goutte => "goutte",
        }
    }
}

impl Default for ShapeId {
    fn default() -> Self {
        Self::Galet
    }
}

fn normalize(mut radii: Vec<f32>, max: f32) -> Vec<f32> {
    let peak = radii.iter().copied().fold(0.0f32, f32::max);
    if peak > 0.0 {
        let k = max / peak;
        for r in &mut radii {
            *r *= k;
        }
    }
    radii
}

fn build_shapes() -> Vec<(ShapeId, Vec<f32>)> {
    let angles: Vec<f32> = (0..PROFILE_SAMPLES)
        .map(|i| (i as f32 / PROFILE_SAMPLES as f32) * TAU)
        .collect();

    let pebble = normalize(
        angles
            .iter()
            .map(|&a| 1.0 + 0.075 * (2.0 * a + 0.5).cos() + 0.035 * (3.0 * a + 2.1).cos())
            .collect(),
        1.02,
    );

    let cloud = normalize(
        union_of_circles_profile(&[
            (-0.44, 0.2, 0.54),
            (0.46, 0.2, 0.5),
            (0.02, 0.3, 0.6),
            (-0.24, -0.3, 0.48),
            (0.3, -0.24, 0.44),
        ]),
        1.02,
    );

    let droplet = normalize(
        profile_from_polygon(&hull_of_circles(0.0, 0.28, 0.66, 0.0, -0.96, 0.05, 96), 0.0, 0.0),
        1.04,
    );

    let capsule = profile_from_polygon(
        &hull_of_circles(-0.42, 0.0, 0.62, 0.42, 0.0, 0.62, 96),
        0.0,
        0.0,
    );

    vec![
        (ShapeId::Cercle, vec![1.0; PROFILE_SAMPLES]),
        (ShapeId::Galet, pebble),
        (ShapeId::Squircle, normalize(superellipse_profile(4.2, 1.0, 1.0), 1.15)),
        (ShapeId::Capsule, capsule),
        (
            ShapeId::Triangle,
            regular_polygon_profile(3, 1.12, 0.34, -90.0),
        ),
        (
            ShapeId::Hexagone,
            regular_polygon_profile(6, 1.04, 0.26, 0.0),
        ),
        (ShapeId::Nuage, cloud),
        (ShapeId::Goutte, droplet),
    ]
}

static SHAPES: OnceLock<Vec<(ShapeId, Vec<f32>)>> = OnceLock::new();

pub fn shape_radii(id: ShapeId) -> Vec<f32> {
    let shapes = SHAPES.get_or_init(build_shapes);
    shapes
        .iter()
        .find(|(shape, _)| *shape == id)
        .map(|(_, radii)| radii.clone())
        .unwrap_or_else(|| vec![1.0; PROFILE_SAMPLES])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorId {
    Encre,
    Creme,
    Brun,
    Rouge,
    Orange,
    Ambre,
    Vert,
    Turquoise,
    Bleu,
    Violet,
    Rose,
    Gris,
}

impl ColorId {
    pub const ALL: [Self; 12] = [
        Self::Encre,
        Self::Creme,
        Self::Brun,
        Self::Rouge,
        Self::Orange,
        Self::Ambre,
        Self::Vert,
        Self::Turquoise,
        Self::Bleu,
        Self::Violet,
        Self::Rose,
        Self::Gris,
    ];

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Encre => "encre",
            Self::Creme => "creme",
            Self::Brun => "brun",
            Self::Rouge => "rouge",
            Self::Orange => "orange",
            Self::Ambre => "ambre",
            Self::Vert => "vert",
            Self::Turquoise => "turquoise",
            Self::Bleu => "bleu",
            Self::Violet => "violet",
            Self::Rose => "rose",
            Self::Gris => "gris",
        }
    }

    pub fn hex(self) -> &'static str {
        match self {
            Self::Encre => "#0a0a0c",
            Self::Creme => "#f1efe9",
            Self::Brun => "#8b5e3c",
            Self::Rouge => "#e8483f",
            Self::Orange => "#f08a24",
            Self::Ambre => "#f0b429",
            Self::Vert => "#3ecf8e",
            Self::Turquoise => "#2fbfa0",
            Self::Bleu => "#3b93f0",
            Self::Violet => "#8b5cf6",
            Self::Rose => "#e152b0",
            Self::Gris => "#a3a3a3",
        }
    }
}

pub fn hex_to_rgba(hex: &str) -> [f32; 4] {
    let value = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0x0a0a0c);
    [
        ((value >> 16) & 255) as f32 / 255.0,
        ((value >> 8) & 255) as f32 / 255.0,
        (value & 255) as f32 / 255.0,
        1.0,
    ]
}

pub fn luminance(rgba: [f32; 4]) -> f32 {
    0.2126 * rgba[0] + 0.7152 * rgba[1] + 0.0722 * rgba[2]
}
