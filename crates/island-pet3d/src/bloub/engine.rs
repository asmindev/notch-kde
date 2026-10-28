use super::decor::{ArcRender, DecorRender, Dot};
use super::expressions::{self, ExpressionId};
use super::face::{blink_at, eye_poses_pair, wander, EyeCfg, EyePose, HeadGaze, Look};
use super::math::{clamp, clamp01, ease_in_out_cubic, lerp, loop_noise, shortest_angle_delta, Rng};
use super::shape::{self, Silhouette};
use super::skins::ShapeId;
use super::states::{self, Pose, StateId};

#[derive(Clone, Debug)]
pub struct BloubFrame {
    pub silhouette: Silhouette,
    pub eyes: [EyePose; 2],
    pub eye_alpha: f32,
    pub body_alpha: f32,
    pub decor: DecorRender,
    pub extent: f32,
    pub state: StateId,
    pub expression: ExpressionId,
    pub shape: ShapeId,
}

pub struct BloubEngine {
    state: StateId,
    state_started: f32,
    expression: Option<ExpressionId>,
    shape: ShapeId,
    shape_from: Silhouette,
    shape_to: Silhouette,
    shape_started: f32,
    shape_duration: f32,
    last_base_sil: Option<Silhouette>,
    last_pose: Option<Pose>,
    look: Look,
    seed: f32,
    time: f32,
    energy: f32,
    compact: bool,
    idle_acting: bool,
    transition_from: Option<Pose>,
    transition_started: f32,
    transition_duration: f32,
}

impl Default for BloubEngine {
    fn default() -> Self {
        let shape = ShapeId::Galet;
        let sil = states::shape_sil(shape);
        Self {
            state: StateId::Idle,
            state_started: 0.0,
            expression: None,
            shape,
            shape_from: sil.clone(),
            shape_to: sil,
            shape_started: 0.0,
            shape_duration: 0.35,
            last_base_sil: None,
            last_pose: None,
            look: Look::default(),
            seed: 1.37,
            time: 0.0,
            energy: 0.0,
            compact: false,
            idle_acting: false,
            transition_from: None,
            transition_started: 0.0,
            transition_duration: 0.32,
        }
    }
}

impl BloubEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> StateId {
        self.state
    }

    pub fn shape(&self) -> ShapeId {
        self.shape
    }

    pub fn expression(&self) -> Option<ExpressionId> {
        self.expression
    }

    pub fn set_state(&mut self, state: StateId, t: f32) {
        if state == self.state {
            return;
        }
        self.change_state(state, t);
    }

    pub fn restart_state(&mut self, state: StateId, t: f32) {
        self.change_state(state, t);
    }

    fn change_state(&mut self, state: StateId, t: f32) {
        let old_state = self.state;
        let old_t = (t - self.state_started).max(0.0);
        let from = self
            .last_pose
            .clone()
            .unwrap_or_else(|| states::pose(old_state, old_t));
        self.transition_from = Some(from);
        self.transition_started = t;
        self.state = state;
        self.state_started = t;
        let target = state.base_body().unwrap_or(self.shape);
        self.start_shape_morph(target, t);
    }

    pub fn set_expression(&mut self, expression: ExpressionId, _t: f32) {
        self.expression = Some(expression);
    }

    pub fn clear_expression(&mut self) {
        self.expression = None;
    }

    pub fn set_shape(&mut self, shape: ShapeId, t: f32) {
        if shape == self.shape {
            return;
        }
        self.shape = shape;
        let target = self.state.base_body().unwrap_or(shape);
        self.start_shape_morph(target, t);
    }

    pub fn set_look(&mut self, x: f32, y: f32, active: bool) {
        self.look = Look {
            x: clamp(x, -1.0, 1.0),
            y: clamp(y, -1.0, 1.0),
            active,
        };
    }

    pub fn set_energy(&mut self, energy: f32) {
        self.energy = clamp01(energy);
    }

    pub fn set_compact(&mut self, compact: bool) {
        self.compact = compact;
    }

    pub fn set_idle_acting(&mut self, active: bool) {
        self.idle_acting = active;
    }

    fn start_shape_morph(&mut self, target: ShapeId, t: f32) {
        let to = states::shape_sil(target);
        if self.shape_to == to {
            return;
        }
        let from = self
            .last_base_sil
            .clone()
            .unwrap_or_else(|| self.shape_to.clone());
        self.shape_from = from;
        self.shape_to = to;
        self.shape_started = t;
        self.shape_duration = 0.35;
    }

    pub fn sample(&mut self, t: f32) -> BloubFrame {
        self.time = t.max(self.time);
        let state_t = (t - self.state_started).max(0.0);
        let mut to_pose = states::pose(self.state, state_t);

        let shape_u = if self.shape_duration <= 0.0 {
            1.0
        } else {
            clamp01((t - self.shape_started) / self.shape_duration)
        };
        let base_sil = shape::blend(&self.shape_from, &self.shape_to, ease_in_out_cubic(shape_u));
        if to_pose.sil.is_none() {
            to_pose.sil = Some(base_sil.clone());
        }

        let mut p = if let Some(from) = self.transition_from.clone() {
            let u = clamp01((t - self.transition_started) / self.transition_duration);
            if u >= 1.0 {
                self.transition_from = None;
                to_pose
            } else {
                blend_pose(&from, &to_pose, ease_in_out_cubic(u))
            }
        } else {
            to_pose
        };

        let expr_id = self.expression.unwrap_or(p.face);
        let expr = expressions::expression(expr_id);
        let mut eye_cfgs = expr.eyes.unwrap_or(p.eyes);
        let mut gaze = if self.expression.is_some() {
            expr.gaze
        } else {
            p.gaze
        };
        p.split = if expr.eyes.is_some() {
            expr.split
        } else {
            p.split
        };
        p.lift += expr.lift;

        if self.compact {
            for eye in &mut eye_cfgs {
                eye.w *= 1.14;
                eye.h *= 1.10;
            }
        }

        if self.idle_acting && expr.eyes.is_none() {
            let act = idle_eye_act(t, self.seed, self.compact);
            for (eye, index) in eye_cfgs.iter_mut().zip(0..2) {
                eye.w = (eye.w * act.w[index]).max(0.02);
                eye.h = (eye.h * act.h[index]).max(0.02);
                eye.open = (eye.open * act.open[index]).clamp(0.06, 1.0);
            }
            gaze.yaw += act.gaze_yaw;
            gaze.pitch += act.gaze_pitch;
        }

        let wander_v = wander(t, self.seed);
        let wander_scale = if self.compact { 1.35 } else { 1.0 };
        if !self.look.active {
            gaze.yaw += wander_v[0] * 12.0 * wander_scale;
            gaze.pitch += wander_v[1] * 10.0 * wander_scale;
        }
        let blink_t = if self.compact { t * 1.35 } else { t };
        let blink = if self.state == StateId::Sleep {
            1.0
        } else {
            blink_at(blink_t, self.seed)
        };
        let mut look = self.look;
        if self.compact {
            look.x = (look.x * 1.35).clamp(-1.0, 1.0);
            look.y = (look.y * 1.35).clamp(-1.0, 1.0);
        }
        let eyes = eye_poses_pair(eye_cfgs, gaze, p.split, look, blink);

        let breathe = loop_noise(t, 4.6, self.seed + 9.1) * 0.010;
        let mut sil = p.sil.clone().unwrap_or(base_sil.clone());
        sil.flip += p.flip;
        sil.cx += p.off_x;
        sil.cy += p.off_y + breathe + p.lift;

        let mut decor = DecorRender::default();
        decor.arcs = p.arcs;
        decor.dots = p.dots;
        decor.notif = p.notif;
        let extent = decor
            .extent()
            .max(1.08)
            .max(sil.radii.iter().copied().fold(0.0f32, f32::max) * 1.08);

        let actual = Pose {
            sil: Some(sil.clone()),
            flip: 0.0,
            off_x: 0.0,
            off_y: 0.0,
            gaze,
            split: p.split,
            eyes: p.eyes,
            eye_alpha: p.eye_alpha,
            body_alpha: p.body_alpha,
            dots: decor.dots.clone(),
            arcs: decor.arcs.clone(),
            notif: decor.notif,
            dots_behind: p.dots_behind,
            face: expr_id,
            lift: 0.0,
        };
        self.last_base_sil = Some(base_sil);
        self.last_pose = Some(actual);

        BloubFrame {
            silhouette: sil,
            eyes,
            eye_alpha: p.eye_alpha,
            body_alpha: p.body_alpha,
            decor,
            extent,
            state: self.state,
            expression: expr_id,
            shape: self.shape,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct EyeAct {
    gaze_yaw: f32,
    gaze_pitch: f32,
    w: [f32; 2],
    h: [f32; 2],
    open: [f32; 2],
}

impl Default for EyeAct {
    fn default() -> Self {
        Self {
            gaze_yaw: 0.0,
            gaze_pitch: 0.0,
            w: [1.0; 2],
            h: [1.0; 2],
            open: [1.0; 2],
        }
    }
}

fn act_pulse(local: f32, start: f32, duration: f32) -> f32 {
    if local < start || local > start + duration {
        return 0.0;
    }
    let u = (local - start) / duration;
    (u * std::f32::consts::PI).sin().clamp(0.0, 1.0)
}

fn idle_eye_act(t: f32, seed: f32, compact: bool) -> EyeAct {
    let period = if compact { 2.8 } else { 4.8 };
    let local = t.rem_euclid(period);
    let cycle = (t / period).floor() as u32;
    let mut rng = Rng::new(seed.to_bits().wrapping_add(cycle.wrapping_mul(0x9e37_79b9)));
    let kind = (rng.next_f32() * 6.0) as u32;
    let start = 0.45 + rng.next_f32() * (period - 1.35);
    let duration = if compact { 0.62 } else { 0.52 };
    let p = act_pulse(local, start, duration);
    if p <= 0.0 {
        return EyeAct::default();
    }
    let strength = if compact { 1.35 } else { 1.0 };
    let mut act = EyeAct::default();
    match kind {
        0 => {
            act.open[0] = 1.0 - 0.86 * p;
            act.h[0] = 1.0 - 0.55 * p;
        }
        1 => {
            let s = 1.0 + 0.20 * p * strength;
            act.w = [s; 2];
            act.h = [s; 2];
        }
        2 => {
            let s = 1.0 - 0.58 * p;
            act.h = [s; 2];
        }
        3 => {
            let side = if cycle % 2 == 0 { 1.0 } else { -1.0 };
            act.gaze_yaw = side * 20.0 * p * strength;
            act.gaze_pitch = -5.0 * p;
        }
        4 => {
            act.open[0] = 1.0 - 0.35 * p;
            act.open[1] = 1.0 - 0.75 * p;
            act.h[1] = 1.0 - 0.45 * p;
        }
        _ => {
            act.w[0] = 1.0 + 0.16 * p;
            act.w[1] = 1.0 - 0.10 * p;
            act.h[0] = 1.0 - 0.25 * p;
            act.h[1] = 1.0 + 0.14 * p;
        }
    }
    act
}

fn blend_eye(a: EyeCfg, b: EyeCfg, u: f32) -> EyeCfg {
    EyeCfg {
        w: lerp(a.w, b.w, u),
        h: lerp(a.h, b.h, u),
        open: lerp(a.open, b.open, u),
        tilt: a.tilt + shortest_angle_delta(a.tilt, b.tilt) * u,
        corner: lerp(a.corner, b.corner, u),
    }
}

fn blend_pose(a: &Pose, b: &Pose, u: f32) -> Pose {
    let sil = match (&a.sil, &b.sil) {
        (Some(x), Some(y)) => Some(shape::blend(x, y, u)),
        (Some(x), None) => Some(x.clone()),
        (None, Some(y)) => Some(y.clone()),
        (None, None) => None,
    };
    let eyes = [blend_eye(a.eyes[0], b.eyes[0], u), blend_eye(a.eyes[1], b.eyes[1], u)];
    Pose {
        sil,
        flip: lerp(a.flip, b.flip, u),
        off_x: lerp(a.off_x, b.off_x, u),
        off_y: lerp(a.off_y, b.off_y, u),
        gaze: HeadGaze {
            yaw: lerp(a.gaze.yaw, b.gaze.yaw, u),
            pitch: lerp(a.gaze.pitch, b.gaze.pitch, u),
            roll: lerp(a.gaze.roll, b.gaze.roll, u),
        },
        split: lerp(a.split, b.split, u),
        eyes,
        eye_alpha: lerp(a.eye_alpha, b.eye_alpha, u),
        body_alpha: lerp(a.body_alpha, b.body_alpha, u),
        dots: blend_dots(&a.dots, &b.dots, u),
        arcs: blend_arcs(&a.arcs, &b.arcs, u),
        notif: if u >= 0.5 { b.notif } else { a.notif },
        dots_behind: if u >= 0.5 { b.dots_behind } else { a.dots_behind },
        face: if u >= 0.5 { b.face } else { a.face },
        lift: lerp(a.lift, b.lift, u),
    }
}

fn blend_dots(a: &[Dot], b: &[Dot], u: f32) -> Vec<Dot> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    for dot in a {
        let mut dot = *dot;
        dot.opacity *= 1.0 - u;
        if dot.opacity > 0.001 {
            out.push(dot);
        }
    }
    for dot in b {
        let mut dot = *dot;
        dot.opacity *= u;
        if dot.opacity > 0.001 {
            out.push(dot);
        }
    }
    out
}

fn blend_arcs(a: &[ArcRender], b: &[ArcRender], u: f32) -> Vec<ArcRender> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    for arc in a {
        let mut arc = arc.clone();
        arc.opacity *= 1.0 - u;
        if arc.opacity > 0.001 {
            out.push(arc);
        }
    }
    for arc in b {
        let mut arc = arc.clone();
        arc.opacity *= u;
        if arc.opacity > 0.001 {
            out.push(arc);
        }
    }
    out
}
