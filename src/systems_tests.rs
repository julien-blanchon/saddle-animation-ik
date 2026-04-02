use bevy::{
    app::{App, Update},
    prelude::*,
};

use crate::{
    FootPlacement, IkChain, IkChainState, IkConstraint, IkDebugSettings, IkJoint, IkPlugin,
    IkSolveSettings, IkSolveStatus, IkSolver, IkTarget, IkTargetSpace, IkWeight, LookAtTarget,
    PoleTarget, ResolvedPole, ResolvedTarget, SolverChainState, SolverJointState,
    compute_root_offset_hint, solve_chain,
};

fn straight_chain() -> SolverChainState {
    SolverChainState {
        joints: vec![
            SolverJointState {
                position: Vec3::ZERO,
                authored_rotation: Quat::IDENTITY,
                settings: IkJoint::default(),
                constraint: None,
            },
            SolverJointState {
                position: Vec3::Y,
                authored_rotation: Quat::IDENTITY,
                settings: IkJoint::default(),
                constraint: None,
            },
            SolverJointState {
                position: Vec3::Y * 2.0,
                authored_rotation: Quat::IDENTITY,
                settings: IkJoint::default(),
                constraint: None,
            },
        ],
        lengths: vec![1.0, 1.0],
    }
}

#[test]
fn reachable_target_converges() {
    let chain = straight_chain();
    let result = solve_chain(
        IkSolver::Fabrik,
        &chain,
        ResolvedTarget {
            position: Vec3::new(0.7, 1.6, 0.2),
            orientation: None,
            position_weight: 1.0,
            rotation_weight: 0.0,
        },
        None,
        IkSolveSettings::default(),
    );

    assert!(result.error < 0.05, "unexpected error: {}", result.error);
    assert_eq!(result.status, IkSolveStatus::Solved);
}

#[test]
fn unreachable_target_extends_without_exploding() {
    let chain = straight_chain();
    let result = solve_chain(
        IkSolver::Fabrik,
        &chain,
        ResolvedTarget {
            position: Vec3::new(0.0, 4.0, 0.0),
            orientation: None,
            position_weight: 1.0,
            rotation_weight: 0.0,
        },
        None,
        IkSolveSettings::default(),
    );

    assert!(result.unreachable);
    assert_eq!(result.status, IkSolveStatus::Unreachable);
    assert!((result.positions[2].length() - 2.0).abs() < 0.001);
    assert!(result.positions.iter().all(|point| point.is_finite()));
}

#[test]
fn ccd_and_fabrik_reduce_error() {
    let chain = straight_chain();
    let target = Vec3::new(0.8, 1.4, 0.3);
    let start_error = chain.positions().last().unwrap().distance(target);
    let fabrik = solve_chain(
        IkSolver::Fabrik,
        &chain,
        ResolvedTarget {
            position: target,
            orientation: None,
            position_weight: 1.0,
            rotation_weight: 0.0,
        },
        None,
        IkSolveSettings::default(),
    );
    let ccd = solve_chain(
        IkSolver::Ccd,
        &chain,
        ResolvedTarget {
            position: target,
            orientation: None,
            position_weight: 1.0,
            rotation_weight: 0.0,
        },
        None,
        IkSolveSettings::default(),
    );

    assert!(fabrik.error < start_error);
    assert!(ccd.error < start_error);
}

#[test]
fn hinge_constraint_is_enforced() {
    let mut chain = straight_chain();
    chain.joints[0].constraint = Some(IkConstraint::Hinge {
        axis: Vec3::Z,
        reference_axis: Vec3::Y,
        min_angle: -0.35,
        max_angle: 0.35,
        strength: 1.0,
    });

    let result = solve_chain(
        IkSolver::Fabrik,
        &chain,
        ResolvedTarget {
            position: Vec3::new(1.5, 0.2, 0.0),
            orientation: None,
            position_weight: 1.0,
            rotation_weight: 0.0,
        },
        None,
        IkSolveSettings::default(),
    );

    let first_dir = (result.positions[1] - result.positions[0]).normalize();
    let angle = Vec3::Y.angle_between(first_dir);
    assert!(angle <= 0.36);
}

#[test]
fn pole_vector_changes_bend_direction_predictably() {
    let chain = straight_chain();
    let upward = solve_chain(
        IkSolver::TwoBone,
        &chain,
        ResolvedTarget {
            position: Vec3::new(0.0, 1.5, 0.0),
            orientation: None,
            position_weight: 1.0,
            rotation_weight: 0.0,
        },
        Some(ResolvedPole {
            point: Vec3::new(1.0, 1.0, 0.0),
            weight: 1.0,
        }),
        IkSolveSettings::default(),
    );
    let downward = solve_chain(
        IkSolver::TwoBone,
        &chain,
        ResolvedTarget {
            position: Vec3::new(0.0, 1.5, 0.0),
            orientation: None,
            position_weight: 1.0,
            rotation_weight: 0.0,
        },
        Some(ResolvedPole {
            point: Vec3::new(-1.0, 1.0, 0.0),
            weight: 1.0,
        }),
        IkSolveSettings::default(),
    );

    assert!(upward.positions[1].x > 0.0);
    assert!(downward.positions[1].x < 0.0);
}

#[test]
fn invalid_chain_fails_gracefully() {
    let chain = SolverChainState {
        joints: vec![SolverJointState {
            position: Vec3::ZERO,
            authored_rotation: Quat::IDENTITY,
            settings: IkJoint::default(),
            constraint: None,
        }],
        lengths: vec![],
    };

    let result = solve_chain(
        IkSolver::Fabrik,
        &chain,
        ResolvedTarget {
            position: Vec3::ONE,
            orientation: None,
            position_weight: 1.0,
            rotation_weight: 0.0,
        },
        None,
        IkSolveSettings::default(),
    );

    assert_eq!(result.status, IkSolveStatus::InvalidChain);
}

#[test]
fn orientation_targets_affect_tip_rotation() {
    let chain = straight_chain();
    let target_orientation = Quat::from_rotation_z(0.6);
    let result = solve_chain(
        IkSolver::Fabrik,
        &chain,
        ResolvedTarget {
            position: Vec3::new(0.6, 1.7, 0.0),
            orientation: Some(target_orientation),
            position_weight: 1.0,
            rotation_weight: 1.0,
        },
        None,
        IkSolveSettings::default(),
    );

    let tip_rotation = *result.rotations.last().unwrap();
    assert!(tip_rotation.dot(target_orientation).abs() > 0.95);
}

#[test]
fn root_offset_hint_handles_steep_targets() {
    let offset = compute_root_offset_hint(
        Vec3::ZERO,
        Vec3::new(0.0, 3.0, 0.25),
        2.0,
        Vec3::Y,
        0.5,
        1.0,
    );
    assert!(offset.y > 0.0);
    assert!(offset.y <= 0.5);
}

#[test]
fn plugin_registration_and_update_mutates_transforms() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<Assets<Scene>>();
    app.insert_resource(IkDebugSettings::default());
    app.add_plugins(IkPlugin::always_on(Update));

    let root = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
            IkJoint::default(),
        ))
        .id();
    let mid = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 1.0, 0.0),
            GlobalTransform::from_xyz(0.0, 1.0, 0.0),
            IkJoint::default(),
        ))
        .id();
    let tip = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 2.0, 0.0),
            GlobalTransform::from_xyz(0.0, 2.0, 0.0),
            IkJoint::default(),
        ))
        .id();

    app.world_mut().spawn((
        IkChain {
            joints: vec![root, mid, tip],
            enabled: true,
            solver: IkSolver::Fabrik,
            solve: IkSolveSettings::default(),
            weight: IkWeight::default(),
        },
        IkTarget {
            enabled: true,
            position: Vec3::new(0.5, 1.6, 0.0),
            orientation: None,
            space: IkTargetSpace::World,
            weight: IkWeight::default(),
        },
    ));

    app.update();
    app.update();

    let tip_transform = app.world().get::<Transform>(tip).unwrap();
    assert!(tip_transform.translation.x > 0.0);

    let state = {
        let world = app.world_mut();
        let mut query = world.query::<&IkChainState>();
        query
            .single(world)
            .expect("chain state should exist")
            .clone()
    };
    assert!(state.last_error < 1.0);
}

#[test]
fn disabled_chains_do_not_mutate_transforms() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(IkPlugin::always_on(Update));

    let root = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
            IkJoint::default(),
        ))
        .id();
    let tip = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 1.0, 0.0),
            GlobalTransform::from_xyz(0.0, 1.0, 0.0),
            IkJoint::default(),
        ))
        .id();

    app.world_mut().spawn((
        IkChain {
            joints: vec![root, tip],
            enabled: false,
            ..default()
        },
        IkTarget {
            position: Vec3::new(3.0, 0.0, 0.0),
            ..default()
        },
    ));

    app.update();
    let tip_transform = app.world().get::<Transform>(tip).unwrap();
    assert!((tip_transform.translation - Vec3::new(0.0, 1.0, 0.0)).length() < 0.001);
}

#[test]
fn zero_weight_preserves_authored_pose() {
    let chain = straight_chain();
    let result = solve_chain(
        IkSolver::Fabrik,
        &chain,
        ResolvedTarget {
            position: Vec3::new(1.5, 0.5, 0.0),
            orientation: None,
            position_weight: 0.0,
            rotation_weight: 0.0,
        },
        None,
        IkSolveSettings::default(),
    );

    assert!((result.positions[2] - Vec3::new(0.0, 2.0, 0.0)).length() < 0.05);
}

#[test]
fn disabled_target_component_preserves_authored_pose() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(IkPlugin::always_on(Update));

    let root = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, 0.0, 0.0), IkJoint::default()))
        .id();
    let mid = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, 1.0, 0.0), IkJoint::default()))
        .id();
    let tip = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, 1.0, 0.0), IkJoint::default()))
        .id();
    app.world_mut().entity_mut(root).add_child(mid);
    app.world_mut().entity_mut(mid).add_child(tip);

    let controller = app
        .world_mut()
        .spawn((
            IkChain {
                joints: vec![root, mid, tip],
                ..default()
            },
            IkTarget {
                enabled: false,
                position: Vec3::new(3.0, 0.0, 0.0),
                ..default()
            },
        ))
        .id();

    app.update();
    app.update();

    let tip_transform = app.world().get::<Transform>(tip).unwrap();
    assert!((tip_transform.translation - Vec3::Y).length() < 0.001);

    let state = app.world().get::<IkChainState>(controller).unwrap();
    assert!(state.last_error < 0.001);
    assert!((state.target_position - state.effector_position).length() < 0.001);
}

#[test]
fn local_space_targets_use_current_entity_transform() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(IkPlugin::always_on(Update));

    let anchor_rotation = Quat::from_rotation_z(0.6);
    let anchor_transform = Transform::from_xyz(1.8, 0.4, 0.2).with_rotation(anchor_rotation);
    let anchor = app
        .world_mut()
        .spawn((anchor_transform, GlobalTransform::IDENTITY))
        .id();

    let root = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, 0.0, 0.0), IkJoint::default()))
        .id();
    let mid = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, 1.0, 0.0), IkJoint::default()))
        .id();
    let tip = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, 1.0, 0.0), IkJoint::default()))
        .id();
    app.world_mut().entity_mut(root).add_child(mid);
    app.world_mut().entity_mut(mid).add_child(tip);

    let local_target = Vec3::new(0.2, 1.5, 0.0);
    let expected_world_target = anchor_transform.transform_point(local_target);
    let controller = app
        .world_mut()
        .spawn((
            IkChain {
                joints: vec![root, mid, tip],
                ..default()
            },
            IkTarget {
                position: local_target,
                orientation: Some(Quat::IDENTITY),
                space: IkTargetSpace::LocalToEntity(anchor),
                ..default()
            },
        ))
        .id();

    app.update();
    app.update();

    let state = app.world().get::<IkChainState>(controller).unwrap();
    assert!((state.target_position - expected_world_target).length() < 0.001);

    let tip_transform = app.world().get::<Transform>(tip).unwrap();
    assert!(tip_transform.rotation.dot(anchor_rotation).abs() > 0.95);
}

#[allow(dead_code)]
fn _api_surface_examples(_foot: FootPlacement, _look_at: LookAtTarget, _pole: PoleTarget) {}
