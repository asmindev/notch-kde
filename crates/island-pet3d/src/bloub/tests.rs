use super::*;

fn assert_frame_finite(frame: &BloubFrame) {
    if frame.eyes_only {
        assert!(frame.extent.is_finite() && frame.extent > 0.0);
    } else {
        assert!(frame.extent.is_finite() && frame.extent >= 1.08);
    }
    assert!(frame.eye_alpha.is_finite() && frame.body_alpha.is_finite());
    for r in &frame.silhouette.radii {
        assert!(r.is_finite() && *r >= 0.0);
    }
    for eye in &frame.eyes {
        assert!(eye.x.is_finite() && eye.y.is_finite());
        assert!(eye.w.is_finite() && eye.h.is_finite());
        assert!(eye.open.is_finite());
        assert!(eye.a.is_finite() && eye.b.is_finite() && eye.c.is_finite() && eye.d.is_finite());
        assert!(eye.depth.is_finite());
        assert!(eye.corner.is_finite() && eye.corner > 0.0);
    }
    assert!(frame.silhouette.flip.is_finite());
    for arc in &frame.decor.arcs {
        assert!(arc.width.is_finite() && arc.opacity.is_finite());
        for p in &arc.points {
            assert!(p.x.is_finite() && p.y.is_finite());
        }
    }
    for dot in &frame.decor.dots {
        assert!(dot.x.is_finite() && dot.y.is_finite() && dot.r.is_finite());
    }
    if let Some(notif) = frame.decor.notif {
        assert!(notif.x.is_finite() && notif.y.is_finite());
    }
}

#[test]
fn every_state_shape_and_expression_is_finite() {
    let mut engine = BloubEngine::new();
    for state in StateId::ALL {
        engine.set_state(state, 0.0);
        for shape in ShapeId::ALL {
            engine.set_shape(shape, 0.0);
            for expression in ExpressionId::ALL {
                engine.set_expression(expression, 0.0);
                for t in [0.0, 0.35, 1.0, 2.5] {
                    let frame = engine.sample(t);
                    assert_frame_finite(&frame);
                }
            }
        }
    }
}

#[test]
fn sample_is_deterministic() {
    let mut a = BloubEngine::new();
    let mut b = BloubEngine::new();
    for engine in [&mut a, &mut b] {
        engine.set_state(StateId::Orbit, 0.0);
        engine.set_shape(ShapeId::Nuage, 0.0);
        engine.set_expression(ExpressionId::Hilare, 0.0);
        engine.set_look(0.4, -0.3, true);
        engine.set_energy(0.7);
    }
    for t in [0.0, 0.4, 1.1, 3.7] {
        let fa = a.sample(t);
        let fb = b.sample(t);
        assert_eq!(fa.extent, fb.extent);
        assert_eq!(fa.silhouette, fb.silhouette);
        assert_eq!(fa.eyes, fb.eyes);
        assert_eq!(fa.decor, fb.decor);
    }
}

#[test]
fn shape_blend_takes_short_rotation_path() {
    let mut a = states::shape_sil(ShapeId::Galet);
    let mut b = states::shape_sil(ShapeId::Galet);
    a.rot = 3.0;
    b.rot = -3.0;
    let mid = shape::blend(&a, &b, 0.5);
    assert!((mid.rot - std::f32::consts::PI).abs() < 0.02);
}

#[test]
fn transient_states_end_without_artifacts() {
    let mut engine = BloubEngine::new();
    for state in [
        StateId::Swirl,
        StateId::Wink,
        StateId::Notify,
        StateId::Ping,
        StateId::Charge,
        StateId::Alert,
        StateId::Burst,
        StateId::Bounce,
        StateId::Shake,
        StateId::Backflip,
        StateId::RotateX,
    ] {
        engine.set_state(state, 0.0);
        let frame = engine.sample(state.duration() + 0.5);
        assert_frame_finite(&frame);
    }
}

#[test]
fn charging_states_use_energy_decor_without_notification_badge() {
    let mut engine = BloubEngine::new();
    for state in [StateId::Charge, StateId::Charging] {
        engine.set_state(state, 0.0);
        let frame = engine.sample(0.5);
        assert_frame_finite(&frame);
        assert!(frame.decor.notif.is_none());
        assert!(!frame.decor.arcs.is_empty());
        assert!(!frame.decor.dots.is_empty());
    }
}

#[test]
fn palette_indices_are_finite_and_wrap() {
    for i in 0..BloubPalette::COUNT * 2 {
        let palette = BloubPalette::from_index(i);
        for c in palette.body.into_iter().chain(palette.eye) {
            assert!(c.is_finite() && (0.0..=1.0).contains(&c));
        }
    }
    assert_eq!(BloubPalette::from_index(0), BloubPalette::LIGHT);
    assert_eq!(
        BloubPalette::from_index(BloubPalette::COUNT),
        BloubPalette::LIGHT
    );
}

#[test]
fn every_shape_has_positive_silhouette() {
    for shape in ShapeId::ALL {
        let radii = skins::shape_radii(shape);
        assert_eq!(radii.len(), profiles::PROFILE_SAMPLES);
        let min = radii.iter().copied().fold(f32::INFINITY, f32::min);
        let max = radii.iter().copied().fold(0.0f32, f32::max);
        assert!(min > 0.15, "{} min radius {min}", shape.name());
        assert!(max > 0.75, "{} max radius {max}", shape.name());
        assert!(max < 1.35, "{} max radius {max}", shape.name());
    }
}

#[test]
fn neutral_eyes_are_tall_sphere_capsules() {
    let cfg = [face::EyeCfg::default(); 2];
    let eyes = face::eye_poses_pair(
        cfg,
        face::REST_GAZE,
        face::EYE_SPLIT,
        face::Look::default(),
        1.0,
    );
    for eye in eyes {
        assert!(eye.h > eye.w * 2.0, "eye should be a tall capsule");
        assert!(eye.depth > 0.2, "resting eyes face the viewer");
        let det = eye.a * eye.d - eye.c * eye.b;
        assert!(
            det > 0.05 && det <= 1.02,
            "projected tangent basis should keep volume"
        );
    }
}

#[test]
fn blink_squashes_eyes_vertically() {
    let cfg = [face::EyeCfg::default(); 2];
    let open = face::eye_poses_pair(
        cfg,
        face::REST_GAZE,
        face::EYE_SPLIT,
        face::Look::default(),
        1.0,
    );
    let closed = face::eye_poses_pair(
        cfg,
        face::REST_GAZE,
        face::EYE_SPLIT,
        face::Look::default(),
        0.08,
    );
    assert!(closed[0].open < open[0].open * 0.2);
    assert!((closed[0].a - open[0].a).abs() < 1e-5);
}

#[test]
fn look_turns_the_head() {
    let cfg = [face::EyeCfg::default(); 2];
    let rest = face::eye_poses_pair(
        cfg,
        face::REST_GAZE,
        face::EYE_SPLIT,
        face::Look::default(),
        1.0,
    );
    let right = face::eye_poses_pair(
        cfg,
        face::REST_GAZE,
        face::EYE_SPLIT,
        face::Look {
            x: 1.0,
            y: 0.0,
            active: true,
        },
        1.0,
    );
    assert!(right[0].x > rest[0].x);
    assert!(right[1].x > rest[1].x);
}

#[test]
fn robot_expression_uses_square_eyes() {
    let robot = expressions::expression(ExpressionId::Robot);
    let eyes = robot.eyes.expect("robot eyes");
    assert!(eyes.iter().all(|eye| eye.corner < 0.25));
}

#[test]
fn expression_eye_shape_morphs_between_expressions() {
    let mut engine = BloubEngine::new();
    engine.set_expression(ExpressionId::Robot, 0.0);
    let robot = engine.sample(0.0);
    let robot_corner = robot.eyes[0].corner;
    let robot_h = robot.eyes[0].h;

    engine.set_expression(ExpressionId::Heureux, 1.0);
    let start = engine.sample(1.0);
    assert!((start.eyes[0].corner - robot_corner).abs() < 1e-4);

    let mid = engine.sample(1.0 + engine::EXPRESSION_MORPH_DURATION * 0.5);
    assert!(mid.eyes[0].corner > robot_corner + 0.05);
    assert!(mid.eyes[0].corner < 0.99);
    assert!(mid.eyes[0].h > 0.17 && mid.eyes[0].h < robot_h);

    let end = engine.sample(1.0 + engine::EXPRESSION_MORPH_DURATION + 0.05);
    assert!(end.eyes[0].corner > 0.99);
    assert!(end.eyes[0].h < robot_h);
}

#[test]
fn backflip_rotates_the_body_around_x() {
    let mut engine = BloubEngine::new();
    engine.set_state(StateId::Backflip, 0.0);
    let frame = engine.sample(0.575);
    assert!(frame.silhouette.flip > 0.5);
    assert_frame_finite(&frame);
}

#[test]
fn compact_mode_enlarges_the_eyes() {
    let mut normal = BloubEngine::new();
    normal.set_state(StateId::Idle, 0.0);
    let normal_frame = normal.sample(1.0);

    let mut compact = BloubEngine::new();
    compact.set_compact(true);
    compact.set_state(StateId::Idle, 0.0);
    let compact_frame = compact.sample(1.0);

    assert!(compact_frame.eyes[0].w > normal_frame.eyes[0].w);
    assert!(compact_frame.eyes[0].h > normal_frame.eyes[0].h);
}

#[test]
fn eyes_only_mode_disables_body_alpha_and_decor_and_zooms_extent() {
    let mut engine = BloubEngine::new();
    engine.set_eyes_only(true);
    let frame = engine.sample(1.0);

    assert!(frame.eyes_only);
    assert_eq!(frame.body_alpha, 0.0);
    assert!(frame.decor.arcs.is_empty());
    assert!(frame.decor.dots.is_empty());
    assert!(frame.extent <= 0.95);
    assert_frame_finite(&frame);

    let t0 = face::eye_transform(&frame.silhouette, &frame.eyes[0]);
    let t1 = face::eye_transform(&frame.silhouette, &frame.eyes[1]);

    // Both eyes must be strictly within the camera extent horizontally and vertically
    assert!(t0.0[0] - t0.2 > -frame.extent && t0.0[0] + t0.2 < frame.extent);
    assert!(t1.0[0] - t1.2 > -frame.extent && t1.0[0] + t1.2 < frame.extent);
    assert!(t0.0[1] - t0.3 > -frame.extent && t0.0[1] + t0.3 < frame.extent);
    assert!(t1.0[1] - t1.3 > -frame.extent && t1.0[1] + t1.3 < frame.extent);

    // Eyes are centered symmetrically around x = 0.0
    let mid_x = (t0.0[0] + t1.0[0]) / 2.0;
    assert!(mid_x.abs() < 0.05);
}
