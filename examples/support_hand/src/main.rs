use saddle_animation_ik_example_support as support;

use std::f32::consts::FRAC_PI_2;
use std::fmt::Write as _;

use bevy::prelude::*;
use saddle_animation_ik::{
    IkChain, IkDebugSettings, IkJoint, IkPlugin, IkTarget, IkTargetAnchor, IkWeight,
};
use saddle_pane::prelude::*;
use support::{setup_scene, spawn_joint_chain};

#[derive(Component)]
struct GripRig;

#[derive(Component)]
struct SupportHandController;

#[derive(Component)]
struct Overlay;

#[derive(Resource, Pane)]
#[pane(title = "IK Support Hand")]
struct SupportHandPane {
    #[pane(tab = "Solve")]
    debug_enabled: bool,
    #[pane(tab = "Solve", slider, min = 1, max = 24)]
    iterations: usize,
    #[pane(tab = "Solve", slider, min = 0.001, max = 0.1, step = 0.001)]
    tolerance: f32,
    #[pane(tab = "Solve", slider, min = 0.0, max = 1.0, step = 0.05)]
    overall_weight: f32,
    #[pane(tab = "Target", slider, min = 0.0, max = 1.0, step = 0.05)]
    rotation_weight: f32,
    #[pane(tab = "Runtime", monitor)]
    grip_error: f32,
}

impl Default for SupportHandPane {
    fn default() -> Self {
        Self {
            debug_enabled: true,
            iterations: 12,
            tolerance: 0.01,
            overall_weight: 0.92,
            rotation_weight: 1.0,
            grip_error: 0.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ik support hand".into(),
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
        .init_resource::<SupportHandPane>()
        .register_pane::<SupportHandPane>()
        .add_plugins(IkPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (animate_grip_rig, sync_support_hand_pane, update_overlay),
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
        Transform::from_xyz(0.0, 4.8, 10.5).looking_at(Vec3::new(1.4, 1.8, 0.0), Vec3::Y),
    );

    let grip_rig = commands
        .spawn((
            Name::new("Grip Rig"),
            GripRig,
            Transform::from_xyz(2.2, 1.9, 0.0),
        ))
        .id();

    let handle_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.72, 0.78),
        metallic: 0.3,
        perceptual_roughness: 0.45,
        ..default()
    });
    let handle_mesh = meshes.add(Cuboid::new(0.18, 0.18, 1.4));
    let grip_mesh = meshes.add(Sphere::new(0.16).mesh().uv(20, 12));
    let grip_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.68, 0.18),
        emissive: Color::srgb(0.45, 0.25, 0.06).into(),
        ..default()
    });

    let handle = commands
        .spawn((
            Name::new("Weapon Body"),
            Mesh3d(handle_mesh),
            MeshMaterial3d(handle_material),
            Transform::IDENTITY,
        ))
        .id();
    let grip_point = commands
        .spawn((
            Name::new("Grip Point"),
            Mesh3d(grip_mesh),
            MeshMaterial3d(grip_material),
            Transform::from_xyz(0.0, -0.04, -0.48),
        ))
        .id();
    commands
        .entity(grip_rig)
        .add_children(&[handle, grip_point]);

    let joints = spawn_joint_chain(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Support Hand",
        Transform::from_xyz(-1.6, 0.9, -0.5),
        &[1.05, 0.9, 0.72],
        Vec3::Y,
        Color::srgb(0.28, 0.84, 0.96),
        IkJoint {
            tip_axis: Vec3::Y,
            pole_axis: Vec3::Z,
            ..default()
        },
    );

    commands.spawn((
        Name::new("Support Hand Controller"),
        SupportHandController,
        IkChain {
            joints,
            weight: IkWeight {
                overall: 0.92,
                position: 1.0,
                rotation: 0.72,
            },
            ..default()
        },
        IkTarget {
            weight: IkWeight {
                overall: 1.0,
                position: 1.0,
                rotation: 1.0,
            },
            ..default()
        },
        IkTargetAnchor {
            entity: grip_point,
            translation_offset: Vec3::ZERO,
            rotation_offset: Quat::from_rotation_x(-FRAC_PI_2),
        },
    ));

    commands.spawn((
        Name::new("Overlay"),
        Overlay,
        Node {
            position_type: PositionType::Absolute,
            left: px(14.0),
            top: px(14.0),
            width: px(400.0),
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

fn animate_grip_rig(time: Res<Time>, mut grip_rig: Single<&mut Transform, With<GripRig>>) {
    let t = time.elapsed_secs();
    grip_rig.translation = Vec3::new(
        2.0 + t.cos() * 0.85,
        1.8 + (t * 1.4).sin() * 0.28,
        (t * 0.8).sin() * 1.35,
    );
    grip_rig.rotation = Quat::from_euler(
        EulerRot::YXZ,
        (t * 0.65).sin() * 0.72,
        (t * 1.15).sin() * 0.18,
        0.0,
    );
}

fn sync_support_hand_pane(
    mut pane: ResMut<SupportHandPane>,
    mut debug: ResMut<IkDebugSettings>,
    mut controller: Single<
        (
            &mut IkChain,
            &mut IkTarget,
            &saddle_animation_ik::IkChainState,
        ),
        With<SupportHandController>,
    >,
) {
    if pane.is_changed() && !pane.is_added() {
        debug.enabled = pane.debug_enabled;
        controller.0.solve.iterations = pane.iterations;
        controller.0.solve.tolerance = pane.tolerance;
        controller.0.weight.overall = pane.overall_weight;
        controller.1.weight.rotation = pane.rotation_weight;
    }

    let pane = pane.bypass_change_detection();
    pane.debug_enabled = debug.enabled;
    pane.iterations = controller.0.solve.iterations;
    pane.tolerance = controller.0.solve.tolerance;
    pane.overall_weight = controller.0.weight.overall;
    pane.rotation_weight = controller.1.weight.rotation;
    pane.grip_error = controller.2.last_error;
}

fn update_overlay(pane: Res<SupportHandPane>, mut overlay: Single<&mut Text, With<Overlay>>) {
    let mut text = String::new();
    let _ = writeln!(text, "SUPPORT HAND IK");
    let _ = writeln!(text, "Moving rig drives a grip point through space");
    let _ = writeln!(text, "Use the pane to tune solve and rotation weights");
    let _ = writeln!(text);
    let _ = writeln!(text, "grip error: {:.3}", pane.grip_error);
    overlay.0 = text;
}
