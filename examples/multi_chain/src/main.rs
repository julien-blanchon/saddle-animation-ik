use saddle_animation_ik_example_support as support;

use std::fmt::Write as _;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use saddle_animation_ik::{IkChain, IkDebugSettings, IkPlugin, IkSolver, IkTarget, IkTargetAnchor};
use saddle_pane::prelude::*;
use support::{OrbitMotion, animate_orbits, setup_scene, spawn_joint_chain, spawn_target};

#[derive(Component)]
struct Overlay;

#[derive(Resource, Pane)]
#[pane(title = "IK Multi Chain")]
struct MultiChainPane {
    #[pane(tab = "Solve")]
    debug_enabled: bool,
    #[pane(tab = "Solve", slider, min = 1, max = 24)]
    iterations: usize,
    #[pane(tab = "Solve", slider, min = 0.001, max = 0.1, step = 0.001)]
    tolerance: f32,
    #[pane(tab = "Solve", slider, min = 0.0, max = 1.0, step = 0.05)]
    overall_weight: f32,
    #[pane(tab = "Runtime", monitor)]
    chain_count: usize,
    #[pane(tab = "Runtime", monitor)]
    max_error: f32,
}

impl Default for MultiChainPane {
    fn default() -> Self {
        Self {
            debug_enabled: false,
            iterations: 12,
            tolerance: 0.01,
            overall_weight: 1.0,
            chain_count: 0,
            max_error: 0.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ik multi chain".into(),
                    resolution: (1440, 900).into(),
                    ..default()
                }),
                ..default()
            }),
            support::pane_plugins(),
        ))
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(IkDebugSettings {
            enabled: false,
            ..default()
        })
        .init_resource::<MultiChainPane>()
        .register_pane::<MultiChainPane>()
        .add_plugins(IkPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (animate_orbits, sync_multi_chain_pane, update_overlay),
        )
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

    commands.spawn((
        Name::new("Overlay"),
        Overlay,
        Node {
            position_type: PositionType::Absolute,
            left: px(14.0),
            top: px(14.0),
            width: px(420.0),
            padding: UiRect::all(px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.07, 0.09, 0.82)),
        Text::new(String::new()),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn sync_multi_chain_pane(
    mut pane: ResMut<MultiChainPane>,
    mut debug: ResMut<IkDebugSettings>,
    mut controllers: Query<(&mut IkChain, &saddle_animation_ik::IkChainState)>,
) {
    let mut chain_count = 0usize;
    let mut max_error = 0.0f32;

    for (mut chain, state) in &mut controllers {
        chain_count += 1;
        max_error = max_error.max(state.last_error);
        if pane.is_changed() && !pane.is_added() {
            chain.solve.iterations = pane.iterations;
            chain.solve.tolerance = pane.tolerance;
            chain.weight.overall = pane.overall_weight;
        }
    }

    if pane.is_changed() && !pane.is_added() {
        debug.enabled = pane.debug_enabled;
    }

    let pane = pane.bypass_change_detection();
    pane.debug_enabled = debug.enabled;
    pane.chain_count = chain_count;
    pane.max_error = max_error;
}

fn update_overlay(pane: Res<MultiChainPane>, mut overlay: Single<&mut Text, With<Overlay>>) {
    let mut text = String::new();
    let _ = writeln!(text, "MULTI-CHAIN IK");
    let _ = writeln!(text, "20 chains mix FABRIK and CCD under one scene");
    let _ = writeln!(text, "Use the pane to change iterations and debug drawing");
    let _ = writeln!(text);
    let _ = writeln!(text, "chains: {}", pane.chain_count);
    let _ = writeln!(text, "max error: {:.3}", pane.max_error);
    overlay.0 = text;
}
