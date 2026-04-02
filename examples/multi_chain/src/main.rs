use saddle_animation_ik_example_support as support;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use saddle_animation_ik::{IkChain, IkDebugSettings, IkPlugin, IkSolver, IkTarget, IkTargetAnchor};
use support::{OrbitMotion, animate_orbits, setup_scene, spawn_joint_chain, spawn_target};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ik multi chain".into(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(IkDebugSettings {
            enabled: false,
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
        Transform::from_xyz(0.0, 10.0, 18.0).looking_at(Vec3::new(0.0, 1.8, 0.0), Vec3::Y),
    );

    for row in 0..4 {
        for col in 0..5 {
            let base = Vec3::new(col as f32 * 3.0 - 6.0, 0.6, row as f32 * 2.8 - 4.2);
            let target = spawn_target(
                &mut commands,
                &mut meshes,
                &mut materials,
                &format!("Target {row}-{col}"),
                base + Vec3::new(1.2, 2.2, 0.0),
                Color::srgb(1.0, 0.4, 0.24),
            );
            commands.entity(target).insert(OrbitMotion {
                center: base + Vec3::new(0.8, 2.0, 0.0),
                radius: Vec3::new(0.8, 0.35, 0.8),
                speed: 0.8 + row as f32 * 0.1 + col as f32 * 0.04,
                phase: row as f32 * 0.5 + col as f32 * 0.2,
            });

            let joints = spawn_joint_chain(
                &mut commands,
                &mut meshes,
                &mut materials,
                &format!("Chain {row}-{col}"),
                Transform::from_translation(base),
                &[0.9, 0.8, 0.7, 0.55],
                Vec3::Y,
                if (row + col) % 2 == 0 {
                    Color::srgb(0.24, 0.82, 0.96)
                } else {
                    Color::srgb(0.32, 0.96, 0.58)
                },
                default(),
            );

            commands.spawn((
                Name::new(format!("Controller {row}-{col}")),
                IkChain {
                    joints,
                    solver: if (row + col) % 2 == 0 {
                        IkSolver::Fabrik
                    } else {
                        IkSolver::Ccd
                    },
                    ..default()
                },
                IkTarget::default(),
                IkTargetAnchor {
                    entity: target,
                    translation_offset: Vec3::ZERO,
                    rotation_offset: Quat::IDENTITY,
                },
            ));
        }
    }
}
