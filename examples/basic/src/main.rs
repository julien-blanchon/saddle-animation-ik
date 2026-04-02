use saddle_animation_ik_example_support as support;

use bevy::prelude::*;
use saddle_animation_ik::{IkChain, IkDebugSettings, IkPlugin, IkTarget, IkTargetAnchor, PoleTarget};
use support::{OrbitMotion, animate_orbits, setup_scene, spawn_joint_chain, spawn_target};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ik basic reach".into(),
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
        .add_systems(Update, animate_orbits)
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
        Transform::from_xyz(-7.0, 5.0, 10.0).looking_at(Vec3::new(-2.0, 1.5, 0.0), Vec3::Y),
    );

    let target = spawn_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Reach Target",
        Vec3::new(1.0, 2.0, 0.0),
        Color::srgb(1.0, 0.32, 0.2),
    );
    commands.entity(target).insert(OrbitMotion {
        center: Vec3::new(-1.0, 2.2, 0.0),
        radius: Vec3::new(1.8, 0.7, 1.1),
        speed: 0.9,
        phase: 0.0,
    });

    let joints = spawn_joint_chain(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Basic Arm",
        Transform::from_xyz(-4.0, 0.8, 0.0),
        &[1.2, 1.0, 0.8],
        Vec3::Y,
        Color::srgb(0.24, 0.78, 0.96),
        default(),
    );

    commands.spawn((
        Name::new("Basic Reach Controller"),
        IkChain {
            joints,
            ..default()
        },
        IkTarget::default(),
        IkTargetAnchor {
            entity: target,
            translation_offset: Vec3::ZERO,
            rotation_offset: Quat::IDENTITY,
        },
        PoleTarget {
            point: Vec3::new(-2.7, 1.6, 2.0),
            ..default()
        },
    ));
}
