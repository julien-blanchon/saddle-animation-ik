use saddle_animation_ik_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_animation_ik::{
    IkChain, IkConstraint, IkDebugSettings, IkJoint, IkPlugin, LookAtTarget,
};
use support::{OrbitMotion, animate_orbits, setup_scene, spawn_joint_chain, spawn_target};

#[derive(Component)]
struct LookTarget;

#[derive(Component)]
struct LookController;

#[derive(Resource, Pane)]
#[pane(title = "IK Look At")]
struct LookPane {
    #[pane(tab = "Solve")]
    debug_enabled: bool,
    #[pane(tab = "Solve", slider, min = 1, max = 24)]
    iterations: usize,
    #[pane(tab = "Solve", slider, min = 0.001, max = 0.1, step = 0.001)]
    tolerance: f32,
    #[pane(tab = "Target", slider, min = 0.5, max = 12.0, step = 0.1)]
    reach_distance: f32,
    #[pane(tab = "Target", slider, min = 0.0, max = 1.0, step = 0.05)]
    rotation_weight: f32,
    #[pane(tab = "Runtime", monitor)]
    look_error: f32,
}

impl Default for LookPane {
    fn default() -> Self {
        Self {
            debug_enabled: true,
            iterations: 12,
            tolerance: 0.01,
            reach_distance: 6.0,
            rotation_weight: 1.0,
            look_error: 0.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ik look at".into(),
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
        .init_resource::<LookPane>()
        .register_pane::<LookPane>()
        .add_plugins(IkPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_orbits, sync_look_target, sync_look_pane))
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

fn sync_look_pane(
    mut pane: ResMut<LookPane>,
    mut debug: ResMut<IkDebugSettings>,
    mut controller: Single<(&mut IkChain, &mut LookAtTarget, &saddle_animation_ik::IkChainState), With<LookController>>,
) {
    if pane.is_changed() && !pane.is_added() {
        debug.enabled = pane.debug_enabled;
        controller.0.solve.iterations = pane.iterations;
        controller.0.solve.tolerance = pane.tolerance;
        controller.1.reach_distance = Some(pane.reach_distance);
        controller.1.weight.rotation = pane.rotation_weight;
    }

    let pane = pane.bypass_change_detection();
    pane.debug_enabled = debug.enabled;
    pane.iterations = controller.0.solve.iterations;
    pane.tolerance = controller.0.solve.tolerance;
    pane.reach_distance = controller.1.reach_distance.unwrap_or(0.0);
    pane.rotation_weight = controller.1.weight.rotation;
    pane.look_error = controller.2.last_error;
}
