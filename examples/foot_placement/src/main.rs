use saddle_animation_ik_example_support as support;

use std::fmt::Write as _;

use bevy::prelude::*;
use saddle_animation_ik::{
    IkChain, IkChainState, IkDebugSettings, IkJoint, IkPlugin, PoleTarget,
    helpers::{FootPlacement, FullBodyIkRig, FullBodyIkRigState, IkRigHelpersPlugin},
};
use saddle_pane::prelude::*;
use support::{setup_scene, spawn_joint_chain, spawn_target};

#[derive(Component)]
struct FootProbe;

#[derive(Component)]
struct Pelvis;

#[derive(Component)]
struct FootController;

#[derive(Component)]
struct Overlay;

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
        .add_plugins((IkPlugin::default(), IkRigHelpersPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (update_foot_contact, sync_foot_pane, update_overlay),
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
        Transform::from_xyz(4.0, 4.0, 10.0).looking_at(Vec3::new(0.5, 1.4, 0.0), Vec3::Y),
    );

    // Stepped terrain with varying heights
    let step_colors = [
        Color::srgb(0.28, 0.24, 0.18),
        Color::srgb(0.32, 0.27, 0.20),
        Color::srgb(0.36, 0.30, 0.22),
        Color::srgb(0.40, 0.33, 0.24),
        Color::srgb(0.44, 0.36, 0.26),
    ];
    let steps: [(f32, f32, f32); 5] = [
        (-3.5, 0.0, -0.8),
        (-1.8, 0.35, -0.4),
        (0.0, 0.55, 0.0),
        (1.8, 0.8, 0.4),
        (3.5, 1.1, 0.8),
    ];
    for (index, (x, y, depth)) in steps.into_iter().enumerate() {
        commands.spawn((
            Name::new(format!("Step {}", index + 1)),
            Mesh3d(meshes.add(Cuboid::new(1.6, 0.35, 1.8))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: step_colors[index],
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
        Vec3::new(-3.5, 0.18, -0.8),
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
                contact_point: Vec3::new(-3.5, 0.18, -0.8),
                contact_normal: Vec3::Y,
                ankle_offset: 0.08,
                foot_up_axis: Vec3::Y,
                foot_forward_axis: Vec3::Z,
                root_offset_hint: Some(default()),
                ..default()
            },
            PoleTarget {
                point: Vec3::new(-1.0, 1.5, 2.0),
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

    // Overlay
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

fn update_foot_contact(
    time: Res<Time>,
    mut probe: Single<&mut Transform, With<FootProbe>>,
    mut controller: Single<
        (&mut FootPlacement, &mut PoleTarget, &IkChainState),
        With<FootController>,
    >,
) {
    // Sweep the foot across the stepped terrain
    let t = time.elapsed_secs() * 0.5;
    let x = t.sin() * 3.8;

    // Determine which step the foot is over
    let steps: [(f32, f32, f32); 5] = [
        (-3.5, 0.0, -0.8),
        (-1.8, 0.35, -0.4),
        (0.0, 0.55, 0.0),
        (1.8, 0.8, 0.4),
        (3.5, 1.1, 0.8),
    ];

    let mut height = 0.18;
    let mut z = 0.0;
    for (sx, sy, sz) in &steps {
        if (x - sx).abs() < 0.9 {
            height = sy + 0.18;
            z = *sz;
            break;
        }
    }

    probe.translation = Vec3::new(x, height, z);
    controller.0.contact_point = probe.translation;
    controller.0.contact_normal = Vec3::Y;
    controller.1.point = Vec3::new(x, height + 1.5, z + 2.0);
}

fn sync_foot_pane(
    mut pane: ResMut<FootPane>,
    mut debug: ResMut<IkDebugSettings>,
    mut controller: Single<(&mut IkChain, &mut FootPlacement, &IkChainState), With<FootController>>,
    mut rig: Single<(&mut FullBodyIkRig, &FullBodyIkRigState)>,
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

fn update_overlay(
    pane: Res<FootPane>,
    rig_state: Single<&FullBodyIkRigState>,
    mut overlay: Single<&mut Text, With<Overlay>>,
) {
    let mut text = String::new();
    let _ = writeln!(text, "FOOT PLACEMENT IK");
    let _ = writeln!(text, "Foot sweeps across stepped terrain");
    let _ = writeln!(text, "Pelvis adjusts to keep foot reachable");
    let _ = writeln!(text);
    let _ = writeln!(text, "foot error: {:.3}", pane.foot_error);
    let _ = writeln!(
        text,
        "root offset: ({:.2}, {:.2}, {:.2})",
        rig_state.combined_root_offset.x,
        rig_state.combined_root_offset.y,
        rig_state.combined_root_offset.z,
    );
    overlay.0 = text;
}
