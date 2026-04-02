use saddle_animation_ik_example_support as support;

use bevy::prelude::*;
use saddle_animation_ik::{FootPlacement, IkChain, IkChainState, IkDebugSettings, IkJoint, IkPlugin};
use support::{setup_scene, spawn_joint_chain, spawn_target};

#[derive(Component)]
struct FootProbe;

#[derive(Component)]
struct Pelvis;

#[derive(Component)]
struct FootController;

const PELVIS_BASE: Vec3 = Vec3::new(-1.0, 2.8, 0.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ik foot placement".into(),
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
        .add_systems(Update, update_foot_contact)
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

    commands.spawn((
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
    ));
}

fn update_foot_contact(
    time: Res<Time>,
    mut probe: Single<&mut Transform, With<FootProbe>>,
    mut controller: Single<(&mut FootPlacement, &IkChainState), With<FootController>>,
    mut pelvis: Single<&mut Transform, (With<Pelvis>, Without<FootProbe>)>,
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
    pelvis.translation = PELVIS_BASE + controller.1.suggested_root_offset;
}
