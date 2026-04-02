use saddle_animation_ik_example_support as support;

use bevy::prelude::*;
use saddle_animation_ik::{
    IkChain, IkConstraint, IkDebugSettings, IkPlugin, IkSolver, IkTarget, IkTargetAnchor,
    PoleTarget,
};
use support::{OrbitMotion, animate_orbits, setup_scene, spawn_joint_chain, spawn_target};

#[derive(Component)]
struct PoleMarker;

#[derive(Component)]
struct TwoBoneController;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ik two bone".into(),
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
        .add_systems(Update, (animate_orbits, sync_pole_target))
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
        Transform::from_xyz(0.0, 4.5, 9.0).looking_at(Vec3::new(0.0, 1.6, 0.0), Vec3::Y),
    );

    let target = spawn_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Two Bone Target",
        Vec3::new(2.0, 2.0, 0.0),
        Color::srgb(1.0, 0.34, 0.26),
    );
    commands.entity(target).insert(OrbitMotion {
        center: Vec3::new(0.2, 2.0, 0.0),
        radius: Vec3::new(2.0, 0.8, 1.5),
        speed: 1.0,
        phase: 0.2,
    });

    let pole = spawn_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Pole Hint",
        Vec3::new(1.6, 1.4, 2.0),
        Color::srgb(0.96, 0.86, 0.28),
    );
    commands.entity(pole).insert((
        PoleMarker,
        OrbitMotion {
            center: Vec3::new(0.0, 1.4, 0.0),
            radius: Vec3::new(1.6, 0.2, 2.4),
            speed: 0.7,
            phase: 1.3,
        },
    ));

    let joints = spawn_joint_chain(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Two Bone Limb",
        Transform::from_xyz(-1.2, 0.6, 0.0),
        &[1.45, 1.2],
        Vec3::Y,
        Color::srgb(0.34, 0.84, 0.95),
        default(),
    );

    commands.entity(joints[0]).insert(IkConstraint::Hinge {
        axis: Vec3::Z,
        reference_axis: Vec3::Y,
        min_angle: -1.25,
        max_angle: 1.15,
        strength: 1.0,
    });

    commands.spawn((
        Name::new("Two Bone Controller"),
        TwoBoneController,
        IkChain {
            joints,
            solver: IkSolver::TwoBone,
            ..default()
        },
        IkTarget::default(),
        IkTargetAnchor {
            entity: target,
            translation_offset: Vec3::ZERO,
            rotation_offset: Quat::IDENTITY,
        },
        PoleTarget {
            point: Vec3::new(1.0, 1.2, 2.0),
            ..default()
        },
    ));
}

fn sync_pole_target(
    pole: Single<&Transform, With<PoleMarker>>,
    mut controller: Single<&mut PoleTarget, With<TwoBoneController>>,
) {
    controller.point = pole.translation;
}
