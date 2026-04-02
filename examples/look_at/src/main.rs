use saddle_animation_ik_example_support as support;

use bevy::prelude::*;
use saddle_animation_ik::{IkChain, IkConstraint, IkDebugSettings, IkJoint, IkPlugin, LookAtTarget};
use support::{OrbitMotion, animate_orbits, setup_scene, spawn_joint_chain, spawn_target};

#[derive(Component)]
struct LookTarget;

#[derive(Component)]
struct LookController;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ik look at".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(IkDebugSettings {
            enabled: true,
            ..default()
        })
        .add_plugins(IkPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_orbits, sync_look_target))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    setup_scene(
        &mut commands,
        &mut meshes,
        &mut materials,
        Transform::from_xyz(0.0, 5.0, 11.0).looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
    );

    let target = spawn_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Look Target",
        Vec3::new(2.0, 2.0, 1.0),
        Color::srgb(1.0, 0.38, 0.2),
    );
    commands.entity(target).insert((
        LookTarget,
        OrbitMotion {
            center: Vec3::new(0.0, 2.0, 0.0),
            radius: Vec3::new(2.4, 1.1, 2.8),
            speed: 0.75,
            phase: 0.5,
        },
    ));

    let joints = spawn_joint_chain(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Look Chain",
        Transform::from_xyz(0.0, 0.8, -2.0),
        &[0.85, 0.8, 0.7],
        Vec3::Z,
        Color::srgb(0.26, 0.8, 0.98),
        IkJoint {
            tip_axis: Vec3::Z,
            pole_axis: Vec3::Y,
            ..default()
        },
    );

    commands.entity(joints[0]).insert(IkConstraint::Cone {
        axis: Vec3::Z,
        max_angle: 1.25,
        strength: 1.0,
    });
    commands.entity(joints[1]).insert(IkConstraint::Cone {
        axis: Vec3::Z,
        max_angle: 0.95,
        strength: 1.0,
    });

    commands.spawn((
        Name::new("Look Controller"),
        LookController,
        IkChain {
            joints,
            ..default()
        },
        LookAtTarget {
            point: Vec3::new(2.0, 2.0, 0.0),
            forward_axis: Vec3::Z,
            up_axis: Vec3::Y,
            ..default()
        },
    ));
}

fn sync_look_target(
    target: Single<&Transform, With<LookTarget>>,
    mut controller: Single<&mut LookAtTarget, With<LookController>>,
) {
    controller.point = target.translation;
}
