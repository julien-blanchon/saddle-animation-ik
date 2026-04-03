use saddle_animation_ik_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_animation_ik::{
    IkChain, IkConstraint, IkDebugSettings, IkPlugin, IkSolver, IkTarget, IkTargetAnchor,
    PoleTarget,
};
use support::{OrbitMotion, animate_orbits, setup_scene, spawn_joint_chain, spawn_target};

#[derive(Component)]
struct PoleMarker;

#[derive(Component)]
struct TwoBoneController;

#[derive(Resource, Pane)]
#[pane(title = "IK Two Bone")]
struct TwoBonePane {
    #[pane(tab = "Solve")]
    debug_enabled: bool,
    #[pane(tab = "Solve", slider, min = 1, max = 24)]
    iterations: usize,
    #[pane(tab = "Solve", slider, min = 0.001, max = 0.1, step = 0.001)]
    tolerance: f32,
    #[pane(tab = "Solve", slider, min = 0.0, max = 1.0, step = 0.05)]
    overall_weight: f32,
    #[pane(tab = "Target", slider, min = 0.0, max = 1.0, step = 0.05)]
    pole_weight: f32,
    #[pane(tab = "Runtime", monitor)]
    reach_error: f32,
}

impl Default for TwoBonePane {
    fn default() -> Self {
        Self {
            debug_enabled: true,
            iterations: 12,
            tolerance: 0.01,
            overall_weight: 1.0,
            pole_weight: 1.0,
            reach_error: 0.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ik two bone".into(),
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
        .init_resource::<TwoBonePane>()
        .register_pane::<TwoBonePane>()
        .add_plugins(IkPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_orbits, sync_pole_target, sync_two_bone_pane))
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

fn sync_two_bone_pane(
    mut pane: ResMut<TwoBonePane>,
    mut debug: ResMut<IkDebugSettings>,
    mut controller: Single<(&mut IkChain, &mut PoleTarget, &saddle_animation_ik::IkChainState), With<TwoBoneController>>,
) {
    if pane.is_changed() && !pane.is_added() {
        debug.enabled = pane.debug_enabled;
        controller.0.solve.iterations = pane.iterations;
        controller.0.solve.tolerance = pane.tolerance;
        controller.0.weight.overall = pane.overall_weight;
        controller.1.weight = pane.pole_weight;
    }

    let pane = pane.bypass_change_detection();
    pane.debug_enabled = debug.enabled;
    pane.iterations = controller.0.solve.iterations;
    pane.tolerance = controller.0.solve.tolerance;
    pane.overall_weight = controller.0.weight.overall;
    pane.pole_weight = controller.1.weight;
    pane.reach_error = controller.2.last_error;
}
