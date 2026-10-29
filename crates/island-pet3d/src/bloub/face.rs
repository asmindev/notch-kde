use super::math::{Rng, clamp01, lerp, loop_noise};
use super::shape::{Silhouette, radius_at_angle};

pub const EYE_SPLIT: f32 = 15.46;
pub const EYE_W: f32 = 0.186;
pub const EYE_H: f32 = 0.412;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadGaze {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

impl HeadGaze {
    pub const fn offset(self, x: f32, y: f32) -> Self {
        Self {
            yaw: self.yaw + x * 45.0,
            pitch: self.pitch + y * 35.0,
            roll: self.roll,
        }
    }
}

pub const REST_GAZE: HeadGaze = HeadGaze {
    yaw: 28.49,
    pitch: 28.62,
    roll: -13.0,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EyeCfg {
    pub w: f32,
    pub h: f32,
    pub open: f32,
    /// Inclinaison propre de la gelule, en radians.
    pub tilt: f32,
    /// 1.0 = capsule arrondie, < 1.0 = coins plus carres (yeux robot).
    pub corner: f32,
}

impl Default for EyeCfg {
    fn default() -> Self {
        Self {
            w: EYE_W,
            h: EYE_H,
            open: 1.0,
            tilt: 0.0,
            corner: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub x: f32,
    pub y: f32,
    pub active: bool,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            active: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EyePose {
    /// Centre de l'oeil dans le repere de la boule (y vers le bas).
    pub x: f32,
    pub y: f32,
    /// Largeur locale de la gelule, en unites de rayon.
    pub w: f32,
    /// Hauteur locale de la gelule, en unites de rayon.
    pub h: f32,
    /// Ecrasement vertical (clignement / paupiere).
    pub open: f32,
    /// Repere tangent de la sphere, projeté à l'écran (y vers le bas).
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    /// Composante z de la normale : <= 0 = face cachée.
    pub depth: f32,
    /// Arrondi des coins de la gelule (1.0 = capsule).
    pub corner: f32,
}

pub(crate) fn eye_transform(
    sil: &Silhouette,
    eye: &EyePose,
) -> ([f32; 2], [f32; 4], f32, f32, f32, bool) {
    let cr = sil.rot.cos();
    let sr = sil.rot.sin();
    let (flip_sin, flip_cos) = sil.flip.sin_cos();
    let fit = radius_at_angle(&sil.radii, eye.y.atan2(eye.x) - sil.rot).clamp(0.2, 2.0);
    let ex = eye.x * fit;
    let ey0 = eye.y * fit;
    let ez0 = eye.depth;
    let ey = ey0 * flip_cos - ez0 * flip_sin;
    let ez = ey0 * flip_sin + ez0 * flip_cos;
    let rx = ex * cr - ey * sr;
    let ry = ex * sr + ey * cr;
    let center = [rx * sil.sx + sil.cx, -(ry * sil.sy + sil.cy)];

    let transform = |x: f32, y: f32| {
        let rx = x * cr - y * sr;
        let ry = x * sr + y * cr;
        [rx * sil.sx, -(ry * sil.sy)]
    };
    let u = transform(eye.a, eye.b * flip_cos);
    let v = transform(eye.c, eye.d * flip_cos);
    let det = u[0] * v[1] - v[0] * u[1];
    let inv = if det.abs() < 1e-6 {
        [1.0, 0.0, 0.0, 1.0]
    } else {
        [v[1] / det, -v[0] / det, -u[1] / det, u[0] / det]
    };

    (
        center,
        inv,
        (eye.w * 0.5).abs().max(1e-4),
        (eye.h * 0.5).abs().max(1e-4),
        eye.open.clamp(0.06, 1.0),
        ez > 0.02,
    )
}

fn blink_window(local: f32, start: f32, dur: f32) -> f32 {
    if local < start || local > start + dur {
        return 1.0;
    }
    let u = (local - start) / dur;
    let tri = 1.0 - (u * 2.0 - 1.0).abs();
    lerp(1.0, 0.06, clamp01(tri))
}

pub fn blink_at(t: f32, seed: f32) -> f32 {
    let period = 2.55 + (seed.fract().abs() * 1.1);
    let local = t.rem_euclid(period);
    let n = (t / period).floor() as u32;
    let mut rng = Rng::new(seed.to_bits().wrapping_add(n.wrapping_mul(0x9e37_79b9)));
    let start = 1.35 + rng.next_f32() * 0.65;
    let dur = 0.14;
    let first = blink_window(local, start, dur);
    let second = if rng.next_f32() < 0.35 {
        blink_window(local, start + dur + 0.18, dur)
    } else {
        1.0
    };
    first.min(second)
}

pub fn wander(t: f32, seed: f32) -> [f32; 2] {
    [
        loop_noise(t, 5.7, seed) * 0.42,
        loop_noise(t, 6.9, seed + 4.3) * 0.30,
    ]
}

fn deg(d: f32) -> f32 {
    d * std::f32::consts::PI / 180.0
}

fn spin(u: [f32; 3], v: [f32; 3], angle: f32) -> ([f32; 3], [f32; 3]) {
    let (s, c) = angle.sin_cos();
    (
        [
            u[0] * c + v[0] * s,
            u[1] * c + v[1] * s,
            u[2] * c + v[2] * s,
        ],
        [
            v[0] * c - u[0] * s,
            v[1] * c - u[1] * s,
            v[2] * c - u[2] * s,
        ],
    )
}

/// Les yeux vivent sur une sphere : chaque oeil récupère le repère tangent de la
/// sphere, projeté en orthographique. La compression et l'inclinaison en
/// découlent, ce qui donne le volume.
pub fn eye_poses_pair(
    cfg: [EyeCfg; 2],
    gaze: HeadGaze,
    split: f32,
    look: Look,
    blink: f32,
) -> [EyePose; 2] {
    let yaw = deg(gaze.yaw) + if look.active { look.x * deg(30.0) } else { 0.0 };
    let pitch = deg(gaze.pitch)
        + if look.active {
            -look.y * deg(22.0)
        } else {
            0.0
        };
    let roll = deg(gaze.roll);

    let mut forward = [0.0, 0.0, 1.0];
    let mut right = [1.0, 0.0, 0.0];
    let mut down = [0.0, 1.0, 0.0];

    (forward, right) = spin(forward, right, yaw);
    (down, forward) = spin(down, forward, pitch);
    (right, down) = spin(right, down, roll);

    let build = |side: f32, cfg: EyeCfg| -> EyePose {
        let (ef, er) = spin(forward, right, deg(split * side));
        let (ts, tc) = cfg.tilt.sin_cos();
        // La gelule est tournée dans son plan tangent : u = largeur, v = hauteur.
        let a = er[0] * tc + down[0] * ts;
        let b = er[1] * tc + down[1] * ts;
        let c = -er[0] * ts + down[0] * tc;
        let d = -er[1] * ts + down[1] * tc;
        let lid = 0.06 + 0.94 * clamp01(blink);
        EyePose {
            x: ef[0],
            y: ef[1],
            w: cfg.w,
            h: cfg.h,
            open: clamp01(cfg.open * lid),
            a,
            b,
            c,
            d,
            depth: ef[2],
            corner: cfg.corner.clamp(0.05, 1.0),
        }
    };

    [build(-1.0, cfg[0]), build(1.0, cfg[1])]
}
