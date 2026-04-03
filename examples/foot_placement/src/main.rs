use saddle_animation_ik_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_animation_ik::{
    FootPlacement, FullBodyIkRig, IkChain, IkChainState, IkDebugSettings, IkJoint, IkPlugin,
};
use support::{setup_scene, spawn_joint_chain, spawn_target};

#[derive(Component)]
struct FootProbe;

#[derive(Component)]
struct Pelvis;

#[derive(Component)]
struct FootController;

const PELVIS_BASE: Vec3 = Vec3::new(-1.0, 2.8, 0.0);

#[derive(Resource, Pane)]
#[pane(title = "IK Foot Placement")]
struct FootPane {
    #[pane(tab = "Solve")]
    debug_enabled: bool,
    #[pane(tab = "Solve", slider, min = 1, max = 24)]
    iterations: usize,
    #[pane(tab = "Solve", slider, min = 0.001, max = 0.1, step = 0.001)]
    tolerance: f32,
    #[pane(tab = "Foot", slider, min = 0.0, max = 0.3, step = 0.01)]
    ankle_offset: f32,
    #[pane(tab = "Foot", slider, min = 0.0, max = 0.8, step = 0.02)]
    max_root_offset: f32,
    #[pane(tab = "Runtime", monitor)]
    foot_error: f32,
    #[pane(tab = "Runtime", monitor)]
    root_offset_y: f32,
}

impl Default for FootPane {
    fn default() -> Self {
        Self {
            debug_enabled: true,
            iterations: 12,
            tolerance: 0.01,
            ankle_offset: 0.08,
            max_root_offset: 0.4,
            foot_error: 0.0,
            root_offset_y: 0.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ik foot placement".into(),
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
        .init_resource::<FootPane>()
        .register_pane::<FootPane>()
        .add_plugins(IkPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (update_foot_contact, sync_foot_pane))
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
        Transform::from_xyz(4.0, 4.0, 10.0).looking_at(Vec3::new(0.5, 1.4, 0.0), Vec3::Y),
    );

    for (index, (x, y, depth)) in [(-2.0, 0.0, -0.8), (0.0, 0.45, 0.0), (2.0, 0.9, 0.8)]
        .into_iter()
        .enumerate()
    {
        commands.spawn((
            Name::new(format!("Step {}", index + 1)),
            Mesh3d(meshes.add(Cuboid::new(1.8, 0.35, 1.8))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.28 + index as f32 * 0.1, 0.24, 0.18),
                perceptual_roughness: 0.95,
                ..default()
            })),
            Transform::from_xyz(x, y, depth),
        ));
    }

    let pelvis = commands
        .spawn((
            Name::new("Pelvis"),
            Pelvis,
            Transform::from_translation(PELVIS_BASE),
        ))
        .id();

    let leg_joint = IkJoint {
        tip_axis: -Vec3::Y,
        pole_axis: Vec3::Z,
        ..default()
    };
    let joints = spawn_joint_chain(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Foot Leg",
        Transform::from_xyz(0.0, 0.0, 0.0),
        &[1.1, 1.0],
        -Vec3::Y,
        Color::srgb(0.24, 0.86, 0.88),
        leg_joint,
    );
    commands.entity(pelvis).add_child(joints[0]);

    let probe = spawn_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Foot Probe",
        Vec3::new(-2.0, 0.18, -0.8),
        Color::srgb(1.0, 0.7, 0.22),
    );
    commands.entity(probe).insert(FootProbe);

    let controller = commands
        .spawn((
        Name::new("Foot Controller"),
        FootController,
        IkChain {
            joints,
            ..default()
        },
        FootPlacement {
            contact_point: Vec3::new(-2.0, 0.18, -0.8),
            contact_normal: Vec3::Y,
            ankle_offset: 0.08,
            foot_up_axis: Vec3::Y,
            foot_forward_axis: Vec3::Z,
            root_offset_hint: Some(default()),
            ..default()
        },
    ))
        .id();
    commands.spawn((
        Name::new("Foot Full Body Rig"),
        FullBodyIkRig::new(pelvis)
            .with_chain(controller)
            .with_root_axis(Vec3::Y)
            .with_max_root_offset(0.4),
    ));
}

fn update_foot_contact(
    time: Res<Time>,
    mut probe: Single<&mut Transform, With<FootProbe>>,
    mut controller: Single<(&mut FootPlacement, &IkChainState), With<FootController>>,
) {
    let x = (time.elapsed_secs() * 0.7).sin() * 2.2;
    let z = if x < -0.7 {
        -0.8
    } else if x < 1.0 {
        0.0
    } else {
        0.8
    };
    let height = if x < -0.7 {
        0.18
    } else if x < 1.0 {
        0.63
    } else {
        1.08
    };

    probe.translation = Vec3::new(x, height, z);
    controller.0.contact_point = probe.translation;
    controller.0.contact_normal = Vec3::Y;
}

fn sync_foot_pane(
    mut pane: ResMut<FootPane>,
    mut debug: ResMut<IkDebugSettings>,
    mut controller: Single<(&mut IkChain, &mut FootPlacement, &IkChainState), With<FootController>>,
    mut rig: Single<(&mut FullBodyIkRig, &saddle_animation_ik::FullBodyIkRigState)>,
) {
    if pane.is_changed() && !pane.is_added() {
        debug.enabled = pane.debug_enabled;
        controller.0.solve.iterations = pane.iterations;
        controller.0.solve.tolerance = pane.tolerance;
        controller.1.ankle_offset = pane.ankle_offset;
        rig.0.max_root_offset = pane.max_root_offset;
    }

    let pane = pane.bypass_change_detection();
    pane.debug_enabled = debug.enabled;
    pane.iterations = controller.0.solve.iterations;
    pane.tolerance = controller.0.solve.tolerance;
    pane.ankle_offset = controller.1.ankle_offset;
    pane.max_root_offset = rig.0.max_root_offset;
    pane.foot_error = controller.2.last_error;
    pane.root_offset_y = rig.1.combined_root_offset.y;
}
