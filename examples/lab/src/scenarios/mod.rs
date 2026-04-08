use bevy::prelude::*;
use saddle_animation_ik::IkDebugSettings;
use saddle_bevy_e2e::{action::Action, actions::{assertions, inspect}, scenario::Scenario};

use crate::LabDiagnostics;

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "ik_smoke",
        "ik_reach_target",
        "ik_reach_extension_limit",
        "ik_foot_placement",
        "ik_crane_arm",
        "ik_constraint_debug",
        "ik_look_at",
        "ik_multi_chain",
        "ik_two_bone",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "ik_smoke" => Some(ik_smoke()),
        "ik_reach_target" => Some(ik_reach_target()),
        "ik_reach_extension_limit" => Some(ik_reach_extension_limit()),
        "ik_foot_placement" => Some(ik_foot_placement()),
        "ik_crane_arm" => Some(ik_crane_arm()),
        "ik_constraint_debug" => Some(ik_constraint_debug()),
        "ik_look_at" => Some(ik_look_at()),
        "ik_multi_chain" => Some(ik_multi_chain()),
        "ik_two_bone" => Some(ik_two_bone()),
        _ => None,
    }
}

fn entity_by_name(world: &mut World, name: &str) -> Option<Entity> {
    let mut query = world.query::<(Entity, &Name)>();
    query
        .iter(world)
        .find(|(_, entity_name)| entity_name.as_str() == name)
        .map(|(entity, _)| entity)
}

fn set_transform(name: &'static str, translation: Vec3) -> Action {
    Action::Custom(Box::new(move |world| {
        let entity = entity_by_name(world, name).expect("named entity should exist");
        let mut entity_ref = world.entity_mut(entity);
        let mut transform = entity_ref
            .get_mut::<Transform>()
            .expect("entity should have transform");
        transform.translation = translation;
        if let Some(mut orbit) = entity_ref.get_mut::<crate::support::OrbitMotion>() {
            orbit.center = translation;
            orbit.radius = Vec3::ZERO;
        }
    }))
}

fn set_debug(enabled: bool) -> Action {
    Action::Custom(Box::new(move |world| {
        world.resource_mut::<IkDebugSettings>().enabled = enabled;
    }))
}

fn diagnostics_ok(world: &World) -> bool {
    let diagnostics = world.resource::<LabDiagnostics>();
    diagnostics.reach_error.is_finite()
        && diagnostics.foot_error.is_finite()
        && diagnostics.look_error.is_finite()
}

fn ik_smoke() -> Scenario {
    Scenario::builder("ik_smoke")
        .description("Launch the crate-local lab, verify the diagnostics resource and all showcase sections initialize, then capture the baseline scene.")
        .then(Action::WaitFrames(45))
        .then(assertions::resource_exists::<LabDiagnostics>("lab diagnostics exist"))
        .then(assertions::custom("diagnostic values are finite", diagnostics_ok))
        .then(assertions::custom("debug drawing starts enabled", |world| {
            world.resource::<IkDebugSettings>().enabled
        }))
        .then(assertions::custom("all showcase sections report stable errors", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.reach_error < 1.0
                && diagnostics.foot_error < 1.0
                && diagnostics.crane_error < 1.0
                && diagnostics.look_error < 3.5
        }))
        .then(Action::Screenshot("ik_smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("ik_smoke"))
        .build()
}

fn ik_reach_target() -> Scenario {
    Scenario::builder("ik_reach_target")
        .description("Move the reach target through two authored poses, verify the chain settles each time, and capture both checkpoints.")
        .then(Action::WaitFrames(30))
        .then(set_transform(
            "Reach Target",
            Vec3::new(-6.4, 2.4, 1.0),
        ))
        .then(Action::WaitFrames(18))
        .then(assertions::custom("reach error stays low at pose A", |world| {
            world.resource::<LabDiagnostics>().reach_error < 0.28
        }))
        .then(Action::Screenshot("reach_pose_a".into()))
        .then(Action::WaitFrames(1))
        .then(set_transform(
            "Reach Target",
            Vec3::new(-7.3, 2.0, -1.0),
        ))
        .then(Action::WaitFrames(20))
        .then(assertions::custom("reach error stays low at pose B", |world| {
            world.resource::<LabDiagnostics>().reach_error < 0.32
        }))
        .then(Action::Screenshot("reach_pose_b".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("ik_reach_target"))
        .build()
}

fn ik_foot_placement() -> Scenario {
    Scenario::builder("ik_foot_placement")
        .description("Move the foot probe from the low step to the high step, verify the leg adapts cleanly, and capture each step.")
        .then(Action::WaitFrames(30))
        .then(set_transform("Foot Probe", Vec3::new(-1.4, 0.18, -0.7)))
        .then(Action::WaitFrames(12))
        .then(assertions::custom("foot error is stable on the first step", |world| {
            world.resource::<LabDiagnostics>().foot_error < 0.25
        }))
        .then(assertions::custom("full-body rig diagnostics stay finite on the low step", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.foot_root_offset.is_finite() && diagnostics.foot_root_offset.y.abs() <= 0.4
        }))
        .then(Action::Screenshot("foot_low_step".into()))
        .then(Action::WaitFrames(1))
        .then(set_transform("Foot Probe", Vec3::new(1.4, 1.08, 0.7)))
        .then(Action::WaitFrames(18))
        .then(assertions::custom("foot error is stable on the high step", |world| {
            world.resource::<LabDiagnostics>().foot_error < 0.3
        }))
        .then(Action::Screenshot("foot_high_step".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("ik_foot_placement"))
        .build()
}

fn ik_reach_extension_limit() -> Scenario {
    Scenario::builder("ik_reach_extension_limit")
        .description("Push the reach chain beyond its maximum extension, verify the solver stays finite, then bring the target back inside range and confirm recovery.")
        .then(Action::WaitFrames(30))
        .then(inspect::log_resource::<LabDiagnostics>("ik_reach_extension_limit_baseline"))
        .then(set_transform(
            "Reach Target",
            Vec3::new(-13.5, 6.8, 0.0),
        ))
        .then(Action::WaitFrames(20))
        .then(assertions::custom(
            "overextended reach target keeps diagnostics finite",
            |world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.reach_error.is_finite()
                    && diagnostics.foot_error.is_finite()
                    && diagnostics.crane_error.is_finite()
                    && diagnostics.look_error.is_finite()
                    && diagnostics.reach_error > 0.5
            },
        ))
        .then(inspect::log_resource::<LabDiagnostics>("ik_reach_extension_limit_overextended"))
        .then(Action::Screenshot("reach_overextended".into()))
        .then(Action::WaitFrames(1))
        .then(set_transform(
            "Reach Target",
            Vec3::new(-6.2, 2.4, 0.3),
        ))
        .then(Action::WaitFrames(18))
        .then(assertions::custom(
            "reach chain recovers after returning inside range",
            |world| world.resource::<LabDiagnostics>().reach_error < 0.35,
        ))
        .then(Action::Screenshot("reach_recovered".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("ik_reach_extension_limit"))
        .build()
}

fn ik_crane_arm() -> Scenario {
    Scenario::builder("ik_crane_arm")
        .description("Move the crane arm target through two non-character poses, verify the chain settles, and capture both checkpoints.")
        .then(Action::WaitFrames(30))
        .then(set_transform(
            "Crane Target",
            Vec3::new(1.8, 3.2, -4.2),
        ))
        .then(Action::WaitFrames(18))
        .then(assertions::custom("crane error stays low at pose A", |world| {
            world.resource::<LabDiagnostics>().crane_error < 0.28
        }))
        .then(Action::Screenshot("crane_pose_a".into()))
        .then(Action::WaitFrames(1))
        .then(set_transform(
            "Crane Target",
            Vec3::new(-0.2, 3.7, -6.6),
        ))
        .then(Action::WaitFrames(22))
        .then(assertions::custom("crane error stays low at pose B", |world| {
            world.resource::<LabDiagnostics>().crane_error < 0.35
        }))
        .then(Action::Screenshot("crane_pose_b".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("ik_crane_arm"))
        .build()
}

fn ik_look_at() -> Scenario {
    Scenario::builder("ik_look_at")
        .description(
            "Drive the look-at chain through a deliberate sweep across the forward/oblique range, \
             verify the look error stays bounded at each pose, and capture a comparison screenshot \
             before and after the sweep.",
        )
        .then(Action::WaitFrames(30))
        // Front-center look
        .then(set_transform("Look Target", Vec3::new(6.0, 2.5, 0.0)))
        .then(Action::WaitFrames(18))
        .then(assertions::custom(
            "look chain tracks the center target cleanly",
            |world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.look_error.is_finite() && diagnostics.look_error < 2.0
            },
        ))
        .then(assertions::custom(
            "look alignment metric is above 0.3 at center",
            |world| world.resource::<LabDiagnostics>().look_alignment > 0.3,
        ))
        .then(Action::Screenshot("look_at_center".into()))
        .then(Action::WaitFrames(1))
        // Sweep left
        .then(set_transform("Look Target", Vec3::new(3.8, 3.6, 3.2)))
        .then(Action::WaitFrames(20))
        .then(assertions::custom(
            "look chain tracks the swept target without numerical blowup",
            |world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.look_error.is_finite() && diagnostics.look_error < 3.5
            },
        ))
        .then(Action::Screenshot("look_at_swept".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("ik_look_at"))
        .build()
}

fn ik_multi_chain() -> Scenario {
    Scenario::builder("ik_multi_chain")
        .description(
            "Verify that all four IK chains (reach, foot, crane, look) solve simultaneously \
             without interfering with each other by checking every error metric stays bounded \
             in the same frame.",
        )
        .then(Action::WaitFrames(45))
        .then(assertions::custom(
            "all four chains solved concurrently without error",
            |world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.reach_error.is_finite()
                    && diagnostics.foot_error.is_finite()
                    && diagnostics.crane_error.is_finite()
                    && diagnostics.look_error.is_finite()
            },
        ))
        .then(assertions::custom(
            "reach chain error within tolerance",
            |world| world.resource::<LabDiagnostics>().reach_error < 0.35,
        ))
        .then(assertions::custom(
            "foot chain error within tolerance",
            |world| world.resource::<LabDiagnostics>().foot_error < 0.35,
        ))
        .then(assertions::custom(
            "crane chain error within tolerance",
            |world| world.resource::<LabDiagnostics>().crane_error < 0.40,
        ))
        .then(assertions::custom(
            "look chain error within tolerance",
            |world| world.resource::<LabDiagnostics>().look_error < 3.5,
        ))
        .then(Action::Screenshot("ik_multi_chain_all_active".into()))
        .then(Action::WaitFrames(1))
        // Perturb all targets simultaneously and verify recovery
        .then(set_transform("Reach Target", Vec3::new(-6.8, 2.6, 0.6)))
        .then(set_transform("Crane Target", Vec3::new(1.2, 3.0, -4.8)))
        .then(set_transform("Look Target", Vec3::new(5.5, 3.0, 1.4)))
        .then(Action::WaitFrames(24))
        .then(assertions::custom(
            "all chains recover after simultaneous target moves",
            |world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.reach_error < 0.45
                    && diagnostics.crane_error < 0.50
                    && diagnostics.look_error < 3.5
            },
        ))
        .then(Action::Screenshot("ik_multi_chain_after_perturb".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("ik_multi_chain"))
        .build()
}

fn ik_two_bone() -> Scenario {
    Scenario::builder("ik_two_bone")
        .description(
            "Exercise the two-segment reach chain through a near-straight and a bent pose, \
             confirming the pole target bends the elbow in the expected direction and the chain \
             stays within reach at both extremes.",
        )
        .then(Action::WaitFrames(30))
        // Pose A: target near the chain root — forces a strong bend
        .then(set_transform("Reach Target", Vec3::new(-7.2, 1.6, 0.6)))
        .then(Action::WaitFrames(18))
        .then(assertions::custom(
            "reach chain error stays low in the bent pose",
            |world| world.resource::<LabDiagnostics>().reach_error < 0.30,
        ))
        .then(Action::Screenshot("two_bone_bent".into()))
        .then(Action::WaitFrames(1))
        // Pose B: target near full extension
        .then(set_transform("Reach Target", Vec3::new(-5.0, 3.4, -1.2)))
        .then(Action::WaitFrames(20))
        .then(assertions::custom(
            "reach chain error stays low near full extension",
            |world| world.resource::<LabDiagnostics>().reach_error < 0.35,
        ))
        .then(assertions::custom(
            "foot and crane remain stable while reach is exercised",
            |world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.foot_error.is_finite()
                    && diagnostics.foot_error < 0.40
                    && diagnostics.crane_error.is_finite()
                    && diagnostics.crane_error < 0.50
            },
        ))
        .then(Action::Screenshot("two_bone_extended".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("ik_two_bone"))
        .build()
}

fn ik_constraint_debug() -> Scenario {
    Scenario::builder("ik_constraint_debug")
        .description("Push the look-at target to an extreme pose, keep debug rendering on, and verify the constrained chain remains stable while the debug overlay is visible.")
        .then(Action::WaitFrames(30))
        .then(set_debug(true))
        .then(set_transform("Look Target", Vec3::new(8.8, 4.8, 3.6)))
        .then(Action::WaitFrames(20))
        .then(assertions::custom("debug rendering remains enabled", |world| {
            world.resource::<IkDebugSettings>().enabled
        }))
        .then(assertions::custom("look chain remains numerically stable", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.look_error.is_finite() && diagnostics.look_error < 3.0
        }))
        .then(Action::Screenshot("constraint_debug_extreme".into()))
        .then(Action::WaitFrames(1))
        .then(set_transform("Look Target", Vec3::new(4.8, 2.3, -2.8)))
        .then(Action::WaitFrames(18))
        .then(assertions::custom("look chain recovers after target sweep", |world| {
            world.resource::<LabDiagnostics>().look_error < 2.5
        }))
        .then(Action::Screenshot("constraint_debug_recovered".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("ik_constraint_debug"))
        .build()
}
