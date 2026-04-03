use saddle_animation_ik_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_animation_ik::{
    IkChain, IkDebugSettings, IkPlugin, IkTarget, IkTargetAnchor, PoleTarget,
};
use support::{OrbitMotion, animate_orbits, setup_scene, spawn_joint_chain, spawn_target};

#[derive(Component)]
struct BasicReachController;

#[derive(Resource, Pane)]
#[pane(title = "IK Basic Reach")]
struct BasicPane {
    #[pane(tab = "Solve")]
    debug_enabled: bool,
    #[pane(tab = "Solve", slider, min = 1, max = 24)]
    iterations: usize,
    #[pane(tab = "Solve", slider, min = 0.001, max = 0.1, step = 0.001)]
    tolerance: f32,
    #[pane(tab = "Solve", slider, min = 0.0, max = 1.0, step = 0.05)]
    overall_weight: f32,
    #[pane(tab = "Runtime", monitor)]
    reach_error: f32,
}

impl Default for BasicPane {
    fn default() -> Self {
        Self {
            debug_enabled: true,
            iterations: 12,
            tolerance: 0.01,
            overall_weight: 1.0,
            reach_error: 0.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ik basic reach".into(),
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            }),
            support::pane_plugins(),
        ))
        .insert_resource(IkDebugSettings {
            enabled: true,
            ..default()
        })
        .init_resource::<BasicPane>()
        .register_pane::<BasicPane>()
        .add_plugins(IkPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_orbits, sync_basic_pane))
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
        BasicReachController,
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

fn sync_basic_pane(
    mut pane: ResMut<BasicPane>,
    mut debug: ResMut<IkDebugSettings>,
    mut controller: Single<(&mut IkChain, &saddle_animation_ik::IkChainState), With<BasicReachController>>,
) {
    if pane.is_changed() && !pane.is_added() {
        debug.enabled = pane.debug_enabled;
        controller.0.solve.iterations = pane.iterations;
        controller.0.solve.tolerance = pane.tolerance;
        controller.0.weight.overall = pane.overall_weight;
    }

    let pane = pane.bypass_change_detection();
    pane.debug_enabled = debug.enabled;
    pane.iterations = controller.0.solve.iterations;
    pane.tolerance = controller.0.solve.tolerance;
    pane.overall_weight = controller.0.weight.overall;
    pane.reach_error = controller.1.last_error;
}
