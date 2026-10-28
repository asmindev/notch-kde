use super::decor::{
    self, colored_dot, dot, tear, ArcRender, NotifRender,
};
use super::expressions::ExpressionId;
use super::face::{EyeCfg, EYE_SPLIT, REST_GAZE};
use super::math::{clamp01, ease_in_out_cubic, ease_out_cubic, TAU};
use super::shape::{silhouette, Silhouette};
use super::skins::{shape_radii, ShapeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateId {
    Idle,
    Swirl,
    Wink,
    Orbit,
    Sleep,
    Notify,
    Ping,
    Charge,
    Charging,
    Play,
    Alert,
    Burst,
    Bounce,
    Shake,
    Egg,
    Hexagon,
    Triangle,
    Backflip,
    RotateX,
}

impl StateId {
    pub const ALL: [Self; 19] = [
        Self::Idle,
        Self::Swirl,
        Self::Wink,
        Self::Orbit,
        Self::Sleep,
        Self::Notify,
        Self::Ping,
        Self::Charge,
        Self::Charging,
        Self::Play,
        Self::Alert,
        Self::Burst,
        Self::Bounce,
        Self::Shake,
        Self::Egg,
        Self::Hexagon,
        Self::Triangle,
        Self::Backflip,
        Self::RotateX,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Swirl => "swirl",
            Self::Wink => "wink",
            Self::Orbit => "orbit",
            Self::Sleep => "sleep",
            Self::Notify => "notify",
            Self::Ping => "ping",
            Self::Charge => "charge",
            Self::Charging => "charging",
            Self::Play => "play",
            Self::Alert => "alert",
            Self::Burst => "burst",
            Self::Bounce => "bounce",
            Self::Shake => "shake",
            Self::Egg => "egg",
            Self::Hexagon => "hexagon",
            Self::Triangle => "triangle",
            Self::Backflip => "backflip",
            Self::RotateX => "rotate-x",
        }
    }

    pub fn base_body(self) -> Option<ShapeId> {
        match self {
            Self::Play | Self::Triangle => Some(ShapeId::Triangle),
            Self::Egg => Some(ShapeId::Goutte),
            Self::Hexagon => Some(ShapeId::Hexagone),
            _ => None,
        }
    }

    pub fn base_face(self) -> ExpressionId {
        match self {
            Self::Swirl | Self::Bounce => ExpressionId::Heureux,
            Self::Wink | Self::Triangle => ExpressionId::Mefiant,
            Self::Orbit | Self::Burst | Self::Backflip => ExpressionId::Surpris,
            Self::RotateX => ExpressionId::Excite,
            Self::Sleep | Self::Egg => ExpressionId::Somnolent,
            Self::Notify | Self::Ping | Self::Charge | Self::Charging => ExpressionId::Excite,
            Self::Play => ExpressionId::Hilare,
            Self::Alert => ExpressionId::Curieux,
            Self::Shake => ExpressionId::Effraye,
            Self::Hexagon => ExpressionId::Blase,
            Self::Idle => ExpressionId::Neutre,
        }
    }

    pub fn duration(self) -> f32 {
        match self {
            Self::Swirl => 1.3,
            Self::Wink => 1.5,
            Self::Notify | Self::Ping => 2.4,
            Self::Charge => 2.8,
            Self::Alert => 2.0,
            Self::Burst => 1.2,
            Self::Bounce => 1.0,
            Self::Shake => 0.8,
            Self::Backflip => 1.15,
            Self::RotateX => 1.8,
            _ => 0.0,
        }
    }

    pub fn loops(self) -> bool {
        self.duration() <= 0.0
    }
}

impl Default for StateId {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Clone, Debug)]
pub struct Pose {
    pub sil: Option<Silhouette>,
    pub flip: f32,
    pub off_x: f32,
    pub off_y: f32,
    pub gaze: super::face::HeadGaze,
    pub split: f32,
    pub eyes: [EyeCfg; 2],
    pub eye_alpha: f32,
    pub body_alpha: f32,
    pub dots: Vec<decor::Dot>,
    pub arcs: Vec<ArcRender>,
    pub notif: Option<NotifRender>,
    pub dots_behind: bool,
    pub face: ExpressionId,
    pub lift: f32,
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            sil: None,
            flip: 0.0,
            off_x: 0.0,
            off_y: 0.0,
            gaze: REST_GAZE,
            split: EYE_SPLIT,
            eyes: [EyeCfg::default(); 2],
            eye_alpha: 1.0,
            body_alpha: 1.0,
            dots: Vec::new(),
            arcs: Vec::new(),
            notif: None,
            dots_behind: false,
            face: ExpressionId::Neutre,
            lift: 0.0,
        }
    }
}

pub fn shape_sil(id: ShapeId) -> Silhouette {
    silhouette(&shape_radii(id), 0.0, 0.0, 0.0, 1.0, 1.0)
}

fn base() -> Pose {
    Pose::default()
}

fn add_ring(pose: &mut Pose, angle: f32, opacity: f32, width: f32, hue: f32, behind: bool) {
    pose.arcs.push(decor::ring(
        decor::RING_A,
        decor::RING_B,
        angle,
        1.45,
        hue,
        opacity,
        width,
        behind,
    ));
    pose.arcs.push(decor::ring(
        decor::RING_B,
        decor::RING_C,
        angle + 2.2,
        1.05,
        hue + 0.12,
        opacity * 0.7,
        width * 0.75,
        behind,
    ));
}

fn add_orbit_decor(pose: &mut Pose, t: f32, opacity: f32) {
    add_ring(pose, t * 0.9, opacity, 0.018, 0.56, true);
    add_ring(pose, -t * 0.7 + 1.4, opacity * 0.8, 0.014, 0.78, true);
    pose.arcs.push(decor::swoosh(
        t * 1.4,
        0.9,
        0.08,
        opacity * 0.9,
        0.024,
    ));
    pose.dots.extend(decor::particles(t, 14, 0x51a3_2b7d, 0.62, true));
    pose.dots_behind = true;
}

fn add_swirl_decor(pose: &mut Pose, t: f32) {
    for i in 0..3 {
        let hue = 0.54 + i as f32 * 0.16;
        pose.arcs.push(decor::arc(
            decor::RING_A + i as f32 * 0.12,
            decor::RING_B + i as f32 * 0.10,
            1.8,
            t * (2.4 + i as f32 * 0.4) + i as f32 * 2.1,
            hue,
            0.55,
            0.020,
            i % 2 == 0,
        ));
    }
    pose.dots.extend(decor::particles(t, 10, 0x7f4a_1c3b, 0.72, true));
    pose.dots_behind = true;
}

fn add_alert_decor(pose: &mut Pose) {
    pose.dots.push(tear(0.0, -0.92, 0.085, 1.0));
    pose.arcs.push(decor::ring(
        decor::RING_A,
        decor::RING_A,
        0.0,
        0.55,
        0.02,
        0.55,
        0.016,
        true,
    ));
}

fn add_notif(pose: &mut Pose) {
    pose.notif = Some(NotifRender {
        x: 0.78,
        y: -0.72,
        r: 0.19,
        notch_r: 0.26,
        color: [0.14, 0.59, 0.91, 1.0],
    });
}

pub fn pose(state: StateId, t: f32) -> Pose {
    let mut p = base();
    p.face = state.base_face();
    match state {
        StateId::Idle => p,
        StateId::Swirl => {
            let u = clamp01(t / 1.3);
            let spin = ease_in_out_cubic(u);
            p.sil = Some(Silhouette {
                rot: spin * TAU * 0.72,
                ..shape_sil(ShapeId::Galet)
            });
            p.off_y = (u * std::f32::consts::PI).sin() * -0.06;
            p.gaze = REST_GAZE.offset(spin * 0.35, -0.05);
            p.eye_alpha = 1.0 - 0.25 * u;
            add_swirl_decor(&mut p, t);
            p
        }
        StateId::Wink => {
            p.face = ExpressionId::Mefiant;
            p.gaze = REST_GAZE.offset(-0.25, -0.05);
            p.off_y = (t * 7.0).sin() * 0.012;
            p
        }
        StateId::Orbit => {
            p.off_y = (t * 2.2).sin() * 0.025;
            p.gaze = REST_GAZE.offset((t * 1.5).cos() * 0.55, (t * 1.1).sin() * 0.32);
            add_orbit_decor(&mut p, t, 0.82);
            p
        }
        StateId::Sleep => {
            for eye in &mut p.eyes {
                eye.open = 0.10;
                eye.h = 0.040;
            }
            p.off_y = 0.03 + (t * 0.9).sin() * 0.012;
            p.gaze = REST_GAZE.offset(0.0, 0.25);
            for i in 0..3 {
                let u = (t * 0.45 + i as f32 * 0.34).fract();
                p.dots.push(dot(
                    0.58 + i as f32 * 0.10,
                    -0.62 - u * 0.28,
                    0.018 + i as f32 * 0.004,
                    (1.0 - u) * 0.65,
                    false,
                ));
            }
            p
        }
        StateId::Notify => {
            let u = clamp01(t / 0.5);
            p.off_y = -(1.0 - ease_out_cubic(u)) * 0.08;
            p.gaze = REST_GAZE.offset(0.25, -0.35);
            add_notif(&mut p);
            p
        }
        StateId::Ping => {
            let u = clamp01(t / 1.15);
            p.off_y = -((u * std::f32::consts::PI * 2.0).sin().abs()) * 0.16;
            p.off_x = (t * 11.0).sin() * 0.025 * (1.0 - u);
            p.gaze = REST_GAZE.offset(0.70, -0.30);
            add_notif(&mut p);
            add_ring(&mut p, 0.0, 0.55 * (1.0 - u), 0.018, 0.56, true);
            p
        }
        StateId::Charge => {
            let u = clamp01(t / 2.6);
            p.off_y = -((t * 5.2).sin().abs()) * 0.12 * (1.0 - u);
            p.off_x = (t * 8.5).sin() * 0.018 * (1.0 - u);
            p.gaze = REST_GAZE.offset(0.18, -0.42);
            add_ring(&mut p, t * 1.5, 0.75 * (1.0 - u), 0.075, 0.33, true);
            p.dots.extend(decor::particles(t, 10, 0x7a31_c0de, 0.14, true));
            for i in 0..3 {
                let phase = i as f32 * 2.1;
                p.dots.push(colored_dot(
                    0.52 + i as f32 * 0.16,
                    -0.88 + (t * 1.4 + phase).sin() * 0.05,
                    0.055,
                    decor::wheel(0.14),
                    0.7,
                ));
            }
            p
        }
        StateId::Charging => {
            let pulse = (t * 1.6).sin() * 0.5 + 0.5;
            p.off_y = (t * 1.25).sin() * 0.04;
            p.gaze = REST_GAZE.offset(0.10, -0.16);
            add_ring(&mut p, t * 0.55, 0.42 + 0.28 * pulse, 0.075, 0.33, true);
            p.dots.push(colored_dot(
                0.52,
                -0.78 + (t * 1.3).sin() * 0.04,
                0.050,
                decor::wheel(0.14),
                0.8,
            ));
            p
        }
        StateId::Play => {
            p.sil = Some(Silhouette {
                rot: (t * 2.6).sin() * 0.18,
                ..shape_sil(ShapeId::Triangle)
            });
            p.off_y = -((t * 4.2).sin().abs()) * 0.10;
            p.gaze = REST_GAZE.offset((t * 3.1).sin() * 0.45, -0.12);
            p.dots.push(colored_dot(
                -0.82,
                -0.42 + (t * 2.0).sin() * 0.08,
                0.055,
                decor::wheel(0.12),
                0.9,
            ));
            p.dots.push(colored_dot(
                0.86,
                0.20 + (t * 2.4 + 1.0).sin() * 0.08,
                0.045,
                decor::wheel(0.72),
                0.85,
            ));
            p
        }
        StateId::Alert => {
            p.off_x = (t * 18.0).sin() * 0.012;
            p.gaze = REST_GAZE.offset(0.0, -0.18);
            add_alert_decor(&mut p);
            p
        }
        StateId::Burst => {
            let u = clamp01(t / 1.2);
            p.off_y = -ease_out_cubic(u) * 0.10;
            p.gaze = REST_GAZE.offset(0.0, -0.25);
            for i in 0..18 {
                let a = i as f32 / 18.0 * TAU + 0.17;
                let rho = 0.55 + ease_out_cubic(u) * 0.75;
                p.dots.push(colored_dot(
                    a.cos() * rho,
                    a.sin() * rho,
                    0.020 + (1.0 - u) * 0.022,
                    decor::wheel(i as f32 / 18.0 + 0.52),
                    (1.0 - u) * 0.95,
                ));
            }
            p
        }
        StateId::Bounce => {
            let u = clamp01(t / 1.0);
            p.off_y = -(u * std::f32::consts::PI * 2.0).sin().abs() * 0.12;
            p.gaze = REST_GAZE.offset(0.0, -0.15);
            add_ring(&mut p, 0.0, 0.45 * (1.0 - u), 0.014, 0.08, true);
            p
        }
        StateId::Shake => {
            let u = clamp01(t / 0.8);
            p.off_x = (t * 32.0).sin() * 0.028 * (1.0 - u);
            p.gaze = REST_GAZE.offset(0.35, 0.10);
            p.arcs.push(decor::ring(
                decor::RING_A,
                decor::RING_B,
                t * 2.0,
                0.8,
                0.98,
                0.45 * (1.0 - u),
                0.012,
                true,
            ));
            p
        }
        StateId::Egg => {
            p.sil = Some(shape_sil(ShapeId::Goutte));
            p.eyes[0].open = 0.55;
            p.eyes[1].open = 0.55;
            p.off_y = (t * 1.1).sin() * 0.018;
            p
        }
        StateId::Hexagon => {
            p.sil = Some(shape_sil(ShapeId::Hexagone));
            p.off_y = (t * 1.4).sin() * 0.02;
            add_ring(&mut p, t * 0.35, 0.35, 0.012, 0.45, true);
            p
        }
        StateId::Triangle => {
            p.sil = Some(shape_sil(ShapeId::Triangle));
            p.off_y = (t * 2.0).sin() * 0.018;
            p.gaze = REST_GAZE.offset(-0.22, 0.05);
            p
        }
        StateId::Backflip => {
            let u = clamp01(t / 1.15);
            let flip = if u >= 1.0 {
                0.0
            } else {
                ease_in_out_cubic(u) * TAU
            };
            p.flip = flip;
            p.off_y = -(u * std::f32::consts::PI).sin() * 0.22;
            p.gaze = REST_GAZE.offset(0.0, -0.22);
            add_ring(&mut p, 0.0, 0.5 * (1.0 - u), 0.014, 0.12, true);
            p
        }
        StateId::RotateX => {
            let u = clamp01(t / 1.8);
            let flip = if u >= 1.0 {
                0.0
            } else {
                ease_in_out_cubic(u) * TAU
            };
            p.flip = flip;
            p.off_y = (t * 3.0).sin() * 0.02;
            p.gaze = REST_GAZE.offset(0.0, -0.18);
            add_ring(&mut p, t * 1.2, 0.35, 0.012, 0.55, true);
            p
        }
    }
}
