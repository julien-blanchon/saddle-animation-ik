use bevy::prelude::*;
use bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};
use saddle_animation_ik::IkDebugSettings;

use crate::LabDiagnostics;

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "ik_smoke",
        "ik_reach_target",
        "ik_foot_placement",
        "ik_constraint_debug",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "ik_smoke" => Some(ik_smoke()),
        "ik_reach_target" => Some(ik_reach_target()),
        "ik_foot_placement" => Some(ik_foot_placement()),
        "ik_constraint_debug" => Some(ik_constraint_debug()),
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
        .description("Move the foot probe from the low step to the high step, verify the leg adapts and the root offset hint becomes positive, and capture each step.")
        .then(Action::WaitFrames(30))
        .then(set_transform("Foot Probe", Vec3::new(-1.4, 0.18, -0.7)))
        .then(Action::WaitFrames(12))
        .then(assertions::custom("foot error is stable on the first step", |world| {
            world.resource::<LabDiagnostics>().foot_error < 0.25
        }))
        .then(assertions::custom("root offset hint reacts on the low step", |world| {
            world.resource::<LabDiagnostics>().foot_root_offset.y.abs() > 0.05
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
