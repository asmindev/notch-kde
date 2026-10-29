use super::math::{Rng, TAU, clamp01};

pub const RING_A: f32 = 0.84;
pub const RING_B: f32 = 1.13;
pub const RING_C: f32 = 1.40;
pub const SWOOSH_A: f32 = 1.27;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcPoint {
    pub x: f32,
    pub y: f32,
    pub color: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArcRender {
    pub points: Vec<ArcPoint>,
    pub width: f32,
    pub opacity: f32,
    pub behind: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dot {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub color: Option<[f32; 4]>,
    pub opacity: f32,
    pub depth: f32,
    pub behind: bool,
    pub tear: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NotifRender {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub notch_r: f32,
    pub color: [f32; 4],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecorRender {
    pub arcs: Vec<ArcRender>,
    pub dots: Vec<Dot>,
    pub notif: Option<NotifRender>,
}

impl DecorRender {
    pub fn push_arc(&mut self, arc: ArcRender) {
        if arc.points.len() >= 2 {
            self.arcs.push(arc);
        }
    }

    pub fn push_dot(&mut self, dot: Dot) {
        if dot.r > 0.0 && dot.opacity > 0.001 {
            self.dots.push(dot);
        }
    }

    pub fn extent(&self) -> f32 {
        let mut extent = 1.02f32;
        for arc in &self.arcs {
            for p in &arc.points {
                extent = extent.max(p.x.abs() + arc.width * 0.5);
                extent = extent.max(p.y.abs() + arc.width * 0.5);
            }
        }
        for dot in &self.dots {
            extent = extent.max(dot.x.abs() + dot.r);
            extent = extent.max(dot.y.abs() + dot.r);
        }
        if let Some(n) = self.notif {
            extent = extent.max(n.x.abs() + n.r + n.notch_r);
            extent = extent.max(n.y.abs() + n.r + n.notch_r);
        }
        extent * 1.06
    }
}

pub fn hsl_to_rgba(h: f32, s: f32, l: f32) -> [f32; 4] {
    let h = h.rem_euclid(1.0);
    let s = clamp01(s);
    let l = clamp01(l);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c * 0.5;
    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m, 1.0]
}

pub fn wheel(hue: f32) -> [f32; 4] {
    hsl_to_rgba(hue, 0.82, 0.58)
}

pub fn arc(
    a: f32,
    b: f32,
    span: f32,
    angle: f32,
    hue: f32,
    opacity: f32,
    width: f32,
    behind: bool,
) -> ArcRender {
    let samples = 72usize;
    let mut points = Vec::with_capacity(samples + 1);
    for i in 0..=samples {
        let t = i as f32 / samples as f32;
        let theta = angle + span * (t - 0.5);
        points.push(ArcPoint {
            x: a * theta.cos(),
            y: b * theta.sin(),
            color: wheel(hue + t * 0.18),
        });
    }
    ArcRender {
        points,
        width,
        opacity,
        behind,
    }
}

pub fn ring(
    a: f32,
    b: f32,
    angle: f32,
    span: f32,
    hue: f32,
    opacity: f32,
    width: f32,
    behind: bool,
) -> ArcRender {
    arc(a, b, span, angle, hue, opacity, width, behind)
}

pub fn swoosh(angle: f32, span: f32, hue: f32, opacity: f32, width: f32) -> ArcRender {
    arc(
        SWOOSH_A,
        SWOOSH_A * 0.90,
        span,
        angle,
        hue,
        opacity,
        width,
        false,
    )
}

pub fn particles(t: f32, count: usize, seed: u32, hue: f32, behind: bool) -> Vec<Dot> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let a = rng.next_f32() * TAU;
        let rho = 1.02 + rng.next_f32() * 0.26;
        let phase = rng.next_f32() * TAU;
        let speed = 0.45 + rng.next_f32() * 0.55;
        let radius = 0.010 + rng.next_f32() * 0.016;
        let wobble = (t * speed + phase).sin() * 0.035;
        let twinkle = 0.55 + 0.45 * (t * 2.1 + phase + i as f32 * 0.7).sin();
        out.push(Dot {
            x: (rho + wobble) * a.cos(),
            y: (rho + wobble) * a.sin(),
            r: radius,
            color: Some(wheel(hue + rng.next_f32() * 0.2)),
            opacity: clamp01(twinkle),
            depth: 0.85,
            behind,
            tear: false,
        });
    }
    out
}

pub fn dot(x: f32, y: f32, r: f32, opacity: f32, behind: bool) -> Dot {
    Dot {
        x,
        y,
        r,
        color: None,
        opacity,
        depth: 1.0,
        behind,
        tear: false,
    }
}

pub fn colored_dot(x: f32, y: f32, r: f32, color: [f32; 4], opacity: f32) -> Dot {
    Dot {
        x,
        y,
        r,
        color: Some(color),
        opacity,
        depth: 1.0,
        behind: false,
        tear: false,
    }
}

pub fn tear(x: f32, y: f32, r: f32, opacity: f32) -> Dot {
    Dot {
        x,
        y,
        r,
        color: None,
        opacity,
        depth: 1.0,
        behind: false,
        tear: true,
    }
}
