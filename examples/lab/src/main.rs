#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use saddle_animation_ik_example_support as support;

use std::fmt::Write as _;

use bevy::prelude::*;
#[cfg(feature = "dev")]
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};
#[cfg(feature = "dev")]
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_pane::prelude::*;
use saddle_animation_ik::{
    FootPlacement, FullBodyIkRig, FullBodyIkRigState, IkChain, IkChainState, IkConstraint,
    IkDebugDraw, IkDebugSettings, IkJoint, IkPlugin, IkTarget, IkTargetAnchor, LookAtTarget,
    PoleTarget,
};
use support::{OrbitMotion, animate_orbits, setup_scene, spawn_joint_chain, spawn_target};

const PELVIS_BASE: Vec3 = Vec3::new(0.0, 2.9, 0.0);

#[derive(Component)]
pub(crate) struct ReachController;

#[derive(Component)]
pub(crate) struct FootController;

#[derive(Component)]
pub(crate) struct LookController;

#[derive(Component)]
pub(crate) struct ReachTarget;

#[derive(Component)]
pub(crate) struct FootProbe;

#[derive(Component)]
pub(crate) struct LookTarget;

#[derive(Component)]
pub(crate) struct PoleMarker;

#[derive(Component)]
pub(crate) struct Pelvis;

#[derive(Component)]
struct Overlay;

#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct LabDiagnostics {
    pub reach_error: f32,
    pub foot_error: f32,
    pub look_error: f32,
    pub foot_root_offset: Vec3,
    pub look_alignment: f32,
    pub debug_enabled: bool,
}

#[derive(Resource, Pane)]
#[pane(title = "IK Lab")]
struct LabPane {
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
    reach_error: f32,
    #[pane(tab = "Runtime", monitor)]
    foot_error: f32,
    #[pane(tab = "Runtime", monitor)]
    look_error: f32,
}

impl Default for LabPane {
    fn default() -> Self {
        Self {
            debug_enabled: true,
            iterations: 12,
            tolerance: 0.01,
            ankle_offset: 0.08,
            max_root_offset: 0.4,
            reach_error: 0.0,
            foot_error: 0.0,
            look_error: 0.0,
        }
    }
}

impl Default for LabDiagnostics {
    fn default() -> Self {
        Self {
            reach_error: 0.0,
            foot_error: 0.0,
            look_error: 0.0,
            foot_root_offset: Vec3::ZERO,
            look_alignment: 0.0,
            debug_enabled: true,
        }
    }
}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.04, 0.045, 0.055)));
    app.insert_resource(IkDebugSettings {
        enabled: true,
        ..default()
    });
    app.init_resource::<LabDiagnostics>();
    app.init_resource::<LabPane>();
    app.register_type::<LabDiagnostics>();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ik crate-local lab".into(),
                resolution: (1520, 860).into(),
                ..default()
            }),
            ..default()
        }),
        support::pane_plugins(),
    ));
    app.register_pane::<LabPane>();
    #[cfg(feature = "dev")]
    app.add_plugins(RemotePlugin::default());
    #[cfg(feature = "dev")]
    app.add_plugins(BrpExtrasPlugin::with_http_plugin(
        RemoteHttpPlugin::default().with_port(lab_brp_port()),
    ));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::IkLabE2EPlugin);
    app.add_plugins(IkPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            animate_orbits,
            sync_pole_target,
            sync_look_target,
            update_foot_contact,
            update_diagnostics,
            sync_lab_pane,
            update_overlay,
        ),
    );
    app.run();
}

#[cfg(feature = "dev")]
fn lab_brp_port() -> u16 {
    std::env::var("BRP_EXTRAS_PORT")
        .or_else(|_| std::env::var("BRP_PORT"))
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(15_712)
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
        Transform::from_xyz(0.5, 7.8, 18.0).looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
    );

    setup_reach_section(&mut commands, &mut meshes, &mut materials);
    setup_foot_section(&mut commands, &mut meshes, &mut materials);
    setup_look_section(&mut commands, &mut meshes, &mut materials);

    commands.spawn((
        Name::new("Lab Overlay"),
        Overlay,
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            width: px(460.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.07, 0.09, 0.82)),
        Text::new(String::new()),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn setup_reach_section(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let target = spawn_target(
        commands,
        meshes,
        materials,
        "Reach Target",
        Vec3::new(-6.0, 2.2, 0.0),
        Color::srgb(1.0, 0.3, 0.2),
    );
    commands.entity(target).insert((
        ReachTarget,
        OrbitMotion {
            center: Vec3::new(-6.0, 2.2, 0.0),
            radius: Vec3::new(1.6, 0.8, 1.2),
            speed: 0.95,
            phase: 0.0,
        },
    ));

    let pole = spawn_target(
        commands,
        meshes,
        materials,
        "Reach Pole",
        Vec3::new(-4.4, 1.8, 2.0),
        Color::srgb(0.98, 0.84, 0.24),
    );
    commands.entity(pole).insert((
        PoleMarker,
        OrbitMotion {
            center: Vec3::new(-5.5, 1.8, 0.0),
            radius: Vec3::new(1.4, 0.2, 2.2),
            speed: 0.55,
            phase: 1.2,
        },
    ));

    let joints = spawn_joint_chain(
        commands,
        meshes,
        materials,
        "Reach Chain",
        Transform::from_xyz(-8.2, 0.8, 0.0),
        &[1.25, 1.0, 0.82],
        Vec3::Y,
        Color::srgb(0.24, 0.82, 0.96),
        default(),
    );

    commands.spawn((
        Name::new("Reach Controller"),
        ReachController,
        IkDebugDraw::default(),
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
            point: Vec3::new(-5.5, 1.8, 2.2),
            ..default()
        },
    ));
}

fn setup_foot_section(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    for (index, (x, y, depth)) in [(-1.4, 0.0, -0.7), (0.0, 0.45, 0.0), (1.4, 0.9, 0.7)]
        .into_iter()
        .enumerate()
    {
        commands.spawn((
            Name::new(format!("Foot Step {}", index + 1)),
            Mesh3d(meshes.add(Cuboid::new(1.2, 0.35, 1.4))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.25 + index as f32 * 0.08, 0.22, 0.18),
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
        commands,
        meshes,
        materials,
        "Foot Chain",
        Transform::IDENTITY,
        &[1.1, 1.0],
        -Vec3::Y,
        Color::srgb(0.28, 0.96, 0.62),
        leg_joint,
    );
    commands.entity(pelvis).add_child(joints[0]);

    let probe = spawn_target(
        commands,
        meshes,
        materials,
        "Foot Probe",
        Vec3::new(-1.4, 0.18, -0.7),
        Color::srgb(1.0, 0.72, 0.18),
    );
    commands.entity(probe).insert(FootProbe);

    let foot_controller = commands
        .spawn((
        Name::new("Foot Controller"),
        FootController,
        IkDebugDraw::default(),
        IkChain {
            joints,
            ..default()
        },
        FootPlacement {
            contact_point: Vec3::new(-1.4, 0.18, -0.7),
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
            .with_chain(foot_controller)
            .with_root_axis(Vec3::Y)
            .with_max_root_offset(0.4),
    ));
}

fn setup_look_section(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let target = spawn_target(
        commands,
        meshes,
        materials,
        "Look Target",
        Vec3::new(6.0, 2.5, 0.0),
        Color::srgb(1.0, 0.36, 0.2),
    );
    commands.entity(target).insert((
        LookTarget,
        OrbitMotion {
            center: Vec3::new(6.0, 2.5, 0.0),
            radius: Vec3::new(2.2, 1.0, 2.6),
            speed: 0.7,
            phase: 0.6,
        },
    ));

    let joints = spawn_joint_chain(
        commands,
        meshes,
        materials,
        "Look Chain",
        Transform::from_xyz(5.2, 0.8, -2.0),
        &[0.85, 0.8, 0.7],
        Vec3::Z,
        Color::srgb(0.3, 0.72, 1.0),
        IkJoint {
            tip_axis: Vec3::Z,
            pole_axis: Vec3::Y,
            ..default()
        },
    );
    commands.entity(joints[0]).insert(IkConstraint::Cone {
        axis: Vec3::Z,
        max_angle: 1.15,
        strength: 1.0,
    });
    commands.entity(joints[1]).insert(IkConstraint::Cone {
        axis: Vec3::Z,
        max_angle: 0.92,
        strength: 1.0,
    });

    commands.spawn((
        Name::new("Look Controller"),
        LookController,
        IkDebugDraw::default(),
        IkChain {
            joints,
            ..default()
        },
        LookAtTarget {
            point: Vec3::new(6.0, 2.5, 0.0),
            forward_axis: Vec3::Z,
            up_axis: Vec3::Y,
            ..default()
        },
    ));
}

fn sync_pole_target(
    pole: Single<&Transform, With<PoleMarker>>,
    mut controller: Single<&mut PoleTarget, With<ReachController>>,
) {
    controller.point = pole.translation;
}

fn sync_look_target(
    target: Single<&Transform, With<LookTarget>>,
    mut controller: Single<&mut LookAtTarget, With<LookController>>,
) {
    controller.point = target.translation;
}

fn update_foot_contact(
    time: Res<Time>,
    mut probe: Single<&mut Transform, With<FootProbe>>,
    mut controller: Single<(&mut FootPlacement, &IkChainState), With<FootController>>,
) {
    let x = (time.elapsed_secs() * 0.65).sin() * 1.45;
    let z = if x < -0.4 {
        -0.7
    } else if x < 0.85 {
        0.0
    } else {
        0.7
    };
    let height = if x < -0.4 {
        0.18
    } else if x < 0.85 {
        0.63
    } else {
        1.08
    };

    probe.translation = Vec3::new(x, height, z);
    controller.0.contact_point = probe.translation;
    controller.0.contact_normal = Vec3::Y;
}

fn update_diagnostics(
    debug: Res<IkDebugSettings>,
    reach_state: Single<&IkChainState, With<ReachController>>,
    foot_state: Single<&IkChainState, With<FootController>>,
    foot_rig: Single<&FullBodyIkRigState>,
    look_state: Single<&IkChainState, With<LookController>>,
    mut diagnostics: ResMut<LabDiagnostics>,
) {
    diagnostics.reach_error = reach_state.last_error;
    diagnostics.foot_error = foot_state.last_error;
    diagnostics.look_error = look_state.last_error;
    diagnostics.foot_root_offset = foot_rig.combined_root_offset;
    diagnostics.debug_enabled = debug.enabled;
    diagnostics.look_alignment = (1.0 / (1.0 + look_state.last_error)).clamp(0.0, 1.0);
}

fn sync_lab_pane(
    mut pane: ResMut<LabPane>,
    mut debug: ResMut<IkDebugSettings>,
    mut reach: Single<
        &mut IkChain,
        (
            With<ReachController>,
            Without<FootController>,
            Without<LookController>,
        ),
    >,
    mut foot: Single<
        (&mut IkChain, &mut FootPlacement),
        (
            With<FootController>,
            Without<ReachController>,
            Without<LookController>,
        ),
    >,
    mut look: Single<
        &mut IkChain,
        (
            With<LookController>,
            Without<ReachController>,
            Without<FootController>,
        ),
    >,
    mut rig: Single<&mut FullBodyIkRig>,
    diagnostics: Res<LabDiagnostics>,
) {
    if pane.is_changed() && !pane.is_added() {
        debug.enabled = pane.debug_enabled;
        reach.solve.iterations = pane.iterations;
        reach.solve.tolerance = pane.tolerance;
        foot.0.solve.iterations = pane.iterations;
        foot.0.solve.tolerance = pane.tolerance;
        look.solve.iterations = pane.iterations;
        look.solve.tolerance = pane.tolerance;
        foot.1.ankle_offset = pane.ankle_offset;
        rig.max_root_offset = pane.max_root_offset;
    }

    let pane = pane.bypass_change_detection();
    pane.debug_enabled = debug.enabled;
    pane.iterations = reach.solve.iterations;
    pane.tolerance = reach.solve.tolerance;
    pane.ankle_offset = foot.1.ankle_offset;
    pane.max_root_offset = rig.max_root_offset;
    pane.reach_error = diagnostics.reach_error;
    pane.foot_error = diagnostics.foot_error;
    pane.look_error = diagnostics.look_error;
}

fn update_overlay(
    diagnostics: Res<LabDiagnostics>,
    mut overlay: Single<&mut Text, With<Overlay>>,
) {
    let mut text = String::new();
    let _ = writeln!(text, "ik crate-local lab");
    let _ = writeln!(text, "reach error: {:.3}", diagnostics.reach_error);
    let _ = writeln!(text, "foot error: {:.3}", diagnostics.foot_error);
    let _ = writeln!(
        text,
        "root offset: ({:.2}, {:.2}, {:.2})",
        diagnostics.foot_root_offset.x,
        diagnostics.foot_root_offset.y,
        diagnostics.foot_root_offset.z
    );
    let _ = writeln!(text, "look error: {:.3}", diagnostics.look_error);
    let _ = writeln!(text, "debug enabled: {}", diagnostics.debug_enabled);
    overlay.0 = text;
}
