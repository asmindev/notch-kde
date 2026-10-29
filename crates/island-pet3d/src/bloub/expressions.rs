use super::face::{EYE_SPLIT, EyeCfg, HeadGaze, REST_GAZE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpressionId {
    Neutre,
    Attentif,
    Surpris,
    Excite,
    Heureux,
    Hilare,
    Colere,
    Triste,
    Effraye,
    Mefiant,
    Confus,
    Curieux,
    Fier,
    Timide,
    Blase,
    Somnolent,
    Robot,
}

impl ExpressionId {
    pub const ALL: [Self; 17] = [
        Self::Neutre,
        Self::Attentif,
        Self::Surpris,
        Self::Excite,
        Self::Heureux,
        Self::Hilare,
        Self::Colere,
        Self::Triste,
        Self::Effraye,
        Self::Mefiant,
        Self::Confus,
        Self::Curieux,
        Self::Fier,
        Self::Timide,
        Self::Blase,
        Self::Somnolent,
        Self::Robot,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&e| e == self).unwrap_or(0)
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Neutre => "neutre",
            Self::Attentif => "attentif",
            Self::Surpris => "surpris",
            Self::Excite => "excite",
            Self::Heureux => "heureux",
            Self::Hilare => "hilare",
            Self::Colere => "colere",
            Self::Triste => "triste",
            Self::Effraye => "effraye",
            Self::Mefiant => "mefiant",
            Self::Confus => "confus",
            Self::Curieux => "curieux",
            Self::Fier => "fier",
            Self::Timide => "timide",
            Self::Blase => "blase",
            Self::Somnolent => "somnolent",
            Self::Robot => "robot",
        }
    }
}

impl Default for ExpressionId {
    fn default() -> Self {
        Self::Neutre
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Expression {
    pub eyes: Option<[EyeCfg; 2]>,
    pub gaze: HeadGaze,
    pub split: f32,
    pub lift: f32,
}

const fn eye(w: f32, h: f32, tilt_deg: f32, open: f32) -> EyeCfg {
    eye_with_corner(w, h, tilt_deg, open, 1.0)
}

const fn eye_with_corner(w: f32, h: f32, tilt_deg: f32, open: f32, corner: f32) -> EyeCfg {
    EyeCfg {
        w,
        h,
        open,
        tilt: tilt_deg * std::f32::consts::PI / 180.0,
        corner,
    }
}

const fn pair(w: f32, h: f32, tilt_deg: f32, open: f32) -> [EyeCfg; 2] {
    [eye(w, h, tilt_deg, open), eye(w, h, -tilt_deg, open)]
}

pub fn expression(id: ExpressionId) -> Expression {
    match id {
        ExpressionId::Neutre => Expression {
            eyes: None,
            gaze: REST_GAZE,
            split: EYE_SPLIT,
            lift: 0.0,
        },
        ExpressionId::Attentif => Expression {
            eyes: Some(pair(0.21, 0.44, 0.0, 1.0)),
            gaze: HeadGaze {
                yaw: 4.0,
                pitch: 5.0,
                roll: -4.0,
            },
            split: 16.0,
            lift: 0.0,
        },
        ExpressionId::Surpris => Expression {
            eyes: Some(pair(0.45, 0.47, 0.0, 1.0)),
            gaze: HeadGaze {
                yaw: 3.0,
                pitch: -3.0,
                roll: 0.0,
            },
            split: 19.0,
            lift: 0.0,
        },
        ExpressionId::Excite => Expression {
            eyes: Some(pair(0.4, 0.56, -10.0, 1.0)),
            gaze: HeadGaze {
                yaw: 6.0,
                pitch: -14.0,
                roll: 0.0,
            },
            split: 19.5,
            lift: 0.0,
        },
        ExpressionId::Heureux => Expression {
            eyes: Some(pair(0.27, 0.17, 14.0, 1.0)),
            gaze: HeadGaze {
                yaw: 5.0,
                pitch: 9.0,
                roll: 0.0,
            },
            split: 17.0,
            lift: 0.0,
        },
        ExpressionId::Hilare => Expression {
            eyes: Some(pair(0.34, 0.13, 20.0, 1.0)),
            gaze: HeadGaze {
                yaw: 4.0,
                pitch: 14.0,
                roll: 0.0,
            },
            split: 18.0,
            lift: 0.0,
        },
        ExpressionId::Colere => Expression {
            eyes: Some(pair(0.34, 0.15, 30.0, 1.0)),
            gaze: HeadGaze {
                yaw: 3.0,
                pitch: 7.0,
                roll: 0.0,
            },
            split: 17.0,
            lift: 0.0,
        },
        ExpressionId::Triste => Expression {
            eyes: Some(pair(0.22, 0.4, -28.0, 1.0)),
            gaze: HeadGaze {
                yaw: 3.0,
                pitch: -13.0,
                roll: 0.0,
            },
            split: 16.0,
            lift: 0.0,
        },
        ExpressionId::Effraye => Expression {
            eyes: Some(pair(0.4, 0.6, 0.0, 1.0)),
            gaze: HeadGaze {
                yaw: 2.0,
                pitch: -20.0,
                roll: 0.0,
            },
            split: 20.5,
            lift: 0.0,
        },
        ExpressionId::Mefiant => Expression {
            eyes: Some([eye(0.21, 0.4, 0.0, 1.0), eye(0.22, 0.15, 0.0, 1.0)]),
            gaze: HeadGaze {
                yaw: 12.0,
                pitch: 6.0,
                roll: -6.0,
            },
            split: 16.0,
            lift: 0.0,
        },
        ExpressionId::Confus => Expression {
            eyes: Some([eye(0.2, 0.44, -18.0, 1.0), eye(0.28, 0.17, 14.0, 1.0)]),
            gaze: HeadGaze {
                yaw: -14.0,
                pitch: 3.0,
                roll: 8.0,
            },
            split: 16.5,
            lift: 0.0,
        },
        ExpressionId::Curieux => Expression {
            eyes: Some([eye(0.24, 0.46, -8.0, 1.0), eye(0.2, 0.38, -8.0, 1.0)]),
            gaze: HeadGaze {
                yaw: 16.0,
                pitch: -9.0,
                roll: -15.0,
            },
            split: 16.5,
            lift: 0.0,
        },
        ExpressionId::Fier => Expression {
            eyes: Some(pair(0.3, 0.15, 18.0, 1.0)),
            gaze: HeadGaze {
                yaw: 5.0,
                pitch: 17.0,
                roll: 0.0,
            },
            split: 17.0,
            lift: 0.0,
        },
        ExpressionId::Timide => Expression {
            eyes: Some(pair(0.17, 0.3, 0.0, 1.0)),
            gaze: HeadGaze {
                yaw: -19.0,
                pitch: -14.0,
                roll: -7.0,
            },
            split: 14.0,
            lift: 0.0,
        },
        ExpressionId::Blase => Expression {
            eyes: Some(pair(0.3, 0.12, 0.0, 1.0)),
            gaze: HeadGaze {
                yaw: -22.0,
                pitch: 2.0,
                roll: 0.0,
            },
            split: 16.0,
            lift: 0.0,
        },
        ExpressionId::Somnolent => Expression {
            eyes: Some(pair(0.2, 0.42, 0.0, 0.42)),
            gaze: HeadGaze {
                yaw: 6.0,
                pitch: -9.0,
                roll: -3.0,
            },
            split: 16.0,
            lift: 0.0,
        },
        ExpressionId::Robot => Expression {
            eyes: Some([
                eye_with_corner(0.38, 0.34, 0.0, 1.0, 0.16),
                eye_with_corner(0.38, 0.34, 0.0, 1.0, 0.16),
            ]),
            gaze: HeadGaze {
                yaw: 4.0,
                pitch: 6.0,
                roll: 0.0,
            },
            split: 18.5,
            lift: 0.0,
        },
    }
}
