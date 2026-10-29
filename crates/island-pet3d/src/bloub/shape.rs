use super::math::{TAU, clamp, lerp, shortest_angle_delta};
use super::profiles::PROFILE_SAMPLES;

#[derive(Clone, Debug, PartialEq)]
pub struct Silhouette {
    pub radii: Vec<f32>,
    pub rot: f32,
    /// Rotation around the horizontal screen axis, for flips.
    pub flip: f32,
    pub cx: f32,
    pub cy: f32,
    pub sx: f32,
    pub sy: f32,
}

impl Default for Silhouette {
    fn default() -> Self {
        Self {
            radii: vec![1.0; PROFILE_SAMPLES],
            rot: 0.0,
            flip: 0.0,
            cx: 0.0,
            cy: 0.0,
            sx: 1.0,
            sy: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

pub fn silhouette(radii: &[f32], rot: f32, cx: f32, cy: f32, sx: f32, sy: f32) -> Silhouette {
    let mut out = vec![1.0; PROFILE_SAMPLES];
    let n = radii.len().min(PROFILE_SAMPLES);
    out[..n].copy_from_slice(&radii[..n]);
    Silhouette {
        radii: out,
        rot,
        flip: 0.0,
        cx,
        cy,
        sx,
        sy,
    }
}

pub fn blend(a: &Silhouette, b: &Silhouette, t: f32) -> Silhouette {
    let t = clamp(t, 0.0, 1.0);
    let mut radii = Vec::with_capacity(PROFILE_SAMPLES);
    for i in 0..PROFILE_SAMPLES {
        radii.push(lerp(a.radii[i], b.radii[i], t));
    }
    let d_rot = shortest_angle_delta(a.rot, b.rot);
    Silhouette {
        radii,
        rot: a.rot + d_rot * t,
        flip: lerp(a.flip, b.flip, t),
        cx: lerp(a.cx, b.cx, t),
        cy: lerp(a.cy, b.cy, t),
        sx: lerp(a.sx, b.sx, t),
        sy: lerp(a.sy, b.sy, t),
    }
}

pub fn profile_from_polygon(poly: &[Point], cx: f32, cy: f32) -> Vec<f32> {
    let mut radii = vec![0.0; PROFILE_SAMPLES];
    let n = poly.len();
    if n < 3 {
        return radii;
    }
    for k in 0..PROFILE_SAMPLES {
        let angle = (k as f32 / PROFILE_SAMPLES as f32) * TAU;
        let dx = angle.cos();
        let dy = angle.sin();
        let mut best = 0.0;
        for i in 0..n {
            let a = poly[i];
            let b = poly[(i + 1) % n];
            let ex = b.x - a.x;
            let ey = b.y - a.y;
            let den = dx * ey - dy * ex;
            if den.abs() < 1e-9 {
                continue;
            }
            let px = a.x - cx;
            let py = a.y - cy;
            let t = (px * ey - py * ex) / den;
            let u = (px * dy - py * dx) / den;
            if t > best && (0.0..=1.0).contains(&u) {
                best = t;
            }
        }
        radii[k] = best;
    }
    radii
}

pub fn hull_of_circles(
    x1: f32,
    y1: f32,
    r1: f32,
    x2: f32,
    y2: f32,
    r2: f32,
    steps: usize,
) -> Vec<Point> {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let dist = (dx * dx + dy * dy).sqrt().max(1e-6);
    let base = dy.atan2(dx);
    let spread = ((r1 - r2) / dist).clamp(-1.0, 1.0).acos();
    let mut pts = Vec::with_capacity(steps + 2);
    let half = steps / 2;
    for i in 0..=half {
        let a = base + spread + ((TAU - 2.0 * spread) * i as f32) / half as f32;
        pts.push(Point {
            x: x1 + a.cos() * r1,
            y: y1 + a.sin() * r1,
        });
    }
    for i in 0..=half {
        let a = base - spread + ((2.0 * spread) * i as f32) / half as f32;
        pts.push(Point {
            x: x2 + a.cos() * r2,
            y: y2 + a.sin() * r2,
        });
    }
    pts
}

pub fn superellipse_profile(n: f32, sx: f32, sy: f32) -> Vec<f32> {
    (0..PROFILE_SAMPLES)
        .map(|i| {
            let angle = (i as f32 / PROFILE_SAMPLES as f32) * TAU;
            let c = ((angle.cos() / sx).abs()).powf(n);
            let s = ((angle.sin() / sy).abs()).powf(n);
            (c + s).powf(-1.0 / n)
        })
        .collect()
}

pub fn union_of_circles_profile(circles: &[(f32, f32, f32)]) -> Vec<f32> {
    let mut out = vec![0.0; PROFILE_SAMPLES];
    for i in 0..PROFILE_SAMPLES {
        let angle = (i as f32 / PROFILE_SAMPLES as f32) * TAU;
        let dx = angle.cos();
        let dy = angle.sin();
        let mut best = 0.0f32;
        for &(cx, cy, r) in circles {
            let b = dx * cx + dy * cy;
            let disc = b * b - (cx * cx + cy * cy - r * r);
            if disc < 0.0 {
                continue;
            }
            let t = b + disc.sqrt();
            if t > best {
                best = t;
            }
        }
        out[i] = best;
    }
    out
}

fn rounded_polygon(verts: &[Point], rc: f32, arc_steps: usize) -> Vec<Point> {
    let n = verts.len();
    let mut out = Vec::new();
    let normal = |a: Point, b: Point| {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        (-dx / len).atan2(dy / len)
    };
    for i in 0..n {
        let prev = verts[(i + n - 1) % n];
        let cur = verts[i];
        let next = verts[(i + 1) % n];
        let a0 = normal(prev, cur);
        let a1 = normal(cur, next);
        let mut d = a1 - a0;
        while d > std::f32::consts::PI {
            d -= TAU;
        }
        while d < -std::f32::consts::PI {
            d += TAU;
        }
        for k in 0..=arc_steps {
            let a = a0 + d * k as f32 / arc_steps as f32;
            out.push(Point {
                x: cur.x + a.cos() * rc,
                y: cur.y + a.sin() * rc,
            });
        }
    }
    out
}

pub fn regular_polygon_profile(sides: usize, radius: f32, rc: f32, rotation_deg: f32) -> Vec<f32> {
    let rot = rotation_deg.to_radians();
    let verts: Vec<Point> = (0..sides)
        .map(|i| {
            let a = rot + (i as f32 / sides as f32) * TAU;
            Point {
                x: a.cos() * (radius - rc),
                y: a.sin() * (radius - rc),
            }
        })
        .collect();
    profile_from_polygon(&rounded_polygon(&verts, rc, 10), 0.0, 0.0)
}

/// Rayon du profil dans une direction quelconque, par interpolation entre les
/// deux échantillons voisins.
pub fn radius_at_angle(radii: &[f32], angle: f32) -> f32 {
    let n = radii.len();
    if n == 0 {
        return 1.0;
    }
    let t = ((((angle / TAU) % 1.0) + 1.0) % 1.0) * n as f32;
    let i = t.floor() as usize;
    lerp(radii[i % n], radii[(i + 1) % n], t - i as f32)
}
