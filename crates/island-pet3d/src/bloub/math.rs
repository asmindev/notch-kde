pub const TAU: f32 = std::f32::consts::TAU;

pub fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi)
}

pub fn clamp01(v: f32) -> f32 {
    clamp(v, 0.0, 1.0)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let x = 1.0 - clamp01(t);
    1.0 - x * x * x
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let x = -2.0 * t + 2.0;
        1.0 - x * x * x / 2.0
    }
}

pub fn loop_noise(t: f32, period: f32, seed: f32) -> f32 {
    let p = (t / period) * TAU;
    0.55 * (p + seed).sin()
        + 0.30 * (2.0 * p + seed * 1.7 + 1.1).sin()
        + 0.15 * (3.0 * p + seed * 2.3 + 2.4).sin()
}

pub struct Rng(u32);

impl Rng {
    pub const fn new(seed: u32) -> Self {
        Self(seed)
    }

    pub fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f32) / 4_294_967_296.0
    }
}

pub fn shortest_angle_delta(from: f32, to: f32) -> f32 {
    let mut d = to - from;
    while d > std::f32::consts::PI {
        d -= TAU;
    }
    while d < -std::f32::consts::PI {
        d += TAU;
    }
    d
}
