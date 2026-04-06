use saddle_animation_ik_example_support as support;

use std::fmt::Write as _;

use bevy::prelude::*;
use saddle_animation_ik::{
    IkChain, IkChainState, IkConstraint, IkDebugDraw, IkDebugSettings, IkPlugin, IkSolver,
    IkTarget, IkTargetAnchor, IkWeight, PoleTarget,
    helpers::{
        FootPlacement, FullBodyIkRig, FullBodyIkRigState, IkRigHelpersPlugin, LookAtTarget,
        RootOffsetHint,
    },
};
use saddle_pane::prelude::*;
use support::humanoid::{HumanoidProportions, spawn_capsule_humanoid};
use support::spawn_target;

// ---------- Marker components ----------

#[derive(Component)]
struct Mannequin;

#[derive(Component)]
struct LeftArmController;
#[derive(Component)]
struct RightArmController;
#[derive(Component)]
struct LeftFootController;
#[derive(Component)]
struct RightFootController;
#[derive(Component)]
struct LookController;

#[derive(Component)]
struct LeftHandTarget;
#[derive(Component)]
struct RightHandTarget;
#[derive(Component)]
struct LookTarget;
#[derive(Component)]
struct LeftFootProbe;
#[derive(Component)]
struct RightFootProbe;

#[derive(Component)]
struct Overlay;

#[derive(Component)]
struct MainCamera;

// ---------- Resources ----------

#[derive(Resource, Pane)]
#[pane(title = "Humanoid IK")]
struct HumanoidPane {
    #[pane(tab = "Solve")]
    debug_enabled: bool,
    #[pane(tab = "Solve", slider, min = 1, max = 24)]
    iterations: usize,
    #[pane(tab = "Foot", slider, min = 0.0, max = 0.3, step = 0.01)]
    ankle_offset: f32,
    #[pane(tab = "Foot", slider, min = 0.0, max = 0.6, step = 0.02)]
    max_root_offset: f32,
    #[pane(tab = "Arms", slider, min = 0.0, max = 1.0, step = 0.05)]
    arm_weight: f32,
    #[pane(tab = "Look", slider, min = 0.0, max = 1.0, step = 0.05)]
    look_weight: f32,
    #[pane(tab = "Runtime", monitor)]
    left_foot_error: f32,
    #[pane(tab = "Runtime", monitor)]
    right_foot_error: f32,
    #[pane(tab = "Runtime", monitor)]
    look_error: f32,
}

impl Default for HumanoidPane {
    fn default() -> Self {
        Self {
            debug_enabled: true,
            iterations: 12,
            ankle_offset: 0.03,
            max_root_offset: 0.25,
            arm_weight: 1.0,
            look_weight: 1.0,
            left_foot_error: 0.0,
            right_foot_error: 0.0,
            look_error: 0.0,
        }
    }
}

// ---------- Terrain ----------

const STEP_WIDTH: f32 = 1.6;
const STEP_DEPTH: f32 = 1.6;

struct TerrainStep {
    position: Vec3,
    height: f32,
}

fn terrain_steps() -> [TerrainStep; 5] {
    [
        TerrainStep {
            position: Vec3::new(-3.0, 0.0, 0.0),
            height: 0.0,
        },
        TerrainStep {
            position: Vec3::new(-1.4, 0.15, 0.0),
            height: 0.3,
        },
        TerrainStep {
            position: Vec3::new(0.0, 0.0, 0.0),
            height: 0.0,
        },
        TerrainStep {
            position: Vec3::new(1.4, 0.2, 0.0),
            height: 0.4,
        },
        TerrainStep {
            position: Vec3::new(3.0, 0.0, 0.0),
            height: 0.0,
        },
    ]
}

fn height_at_x(x: f32) -> f32 {
    let steps = terrain_steps();
    for step in &steps {
        let half_w = STEP_WIDTH * 0.5;
        if x >= step.position.x - half_w && x < step.position.x + half_w {
            return step.height;
        }
    }
    0.0
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ik humanoid — full body IK demo".into(),
                    resolution: (1440, 860).into(),
                    ..default()
                }),
                ..default()
            }),
            support::pane_plugins(),
        ))
        .insert_resource(ClearColor(Color::srgb(0.06, 0.065, 0.08)))
        .insert_resource(IkDebugSettings {
            enabled: true,
            ..default()
        })
        .init_resource::<HumanoidPane>()
        .register_pane::<HumanoidPane>()
        .add_plugins((IkPlugin::default(), IkRigHelpersPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                animate_walking,
                update_look_from_mouse,
                sync_humanoid_pane,
                update_overlay,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Name::new("Main Camera"),
        MainCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.5, 6.0).looking_at(Vec3::new(0.0, 0.9, 0.0), Vec3::Y),
    ));

    // Lights
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: 18_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(8.0, 14.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Fill Light"),
        DirectionalLight {
            illuminance: 4_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-6.0, 8.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.14, 0.15, 0.17),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    // Terrain steps
    let step_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.2, 0.18),
        perceptual_roughness: 0.95,
        ..default()
    });
    for (i, step) in terrain_steps().iter().enumerate() {
        if step.height > 0.0 {
            commands.spawn((
                Name::new(format!("Step {}", i)),
                Mesh3d(meshes.add(Cuboid::new(STEP_WIDTH, step.height, STEP_DEPTH))),
                MeshMaterial3d(step_mat.clone()),
                Transform::from_xyz(step.position.x, step.height * 0.5, step.position.z),
            ));
        }
    }

    // Spawn the humanoid
    let proportions = HumanoidProportions::default();
    let joints = spawn_capsule_humanoid(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Mannequin",
        Vec3::new(-3.0, 0.0, 0.0),
        &proportions,
        Color::srgb(0.35, 0.55, 0.75),
        Color::srgb(0.8, 0.85, 0.95),
    );

    commands.entity(joints.root).insert(Mannequin);

    // --- IK targets ---
    // Look target (invisible, follows mouse)
    let look_target = commands
        .spawn((
            Name::new("Look Target"),
            LookTarget,
            Transform::from_xyz(0.0, 1.6, 3.0),
            Visibility::default(),
        ))
        .id();

    // Hand targets
    let left_hand_target = spawn_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "L Hand Target",
        Vec3::new(-3.5, 0.7, 0.3),
        Color::srgb(0.2, 0.9, 0.4),
    );
    commands.entity(left_hand_target).insert(LeftHandTarget);

    let right_hand_target = spawn_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "R Hand Target",
        Vec3::new(-2.5, 0.7, 0.3),
        Color::srgb(0.9, 0.4, 0.2),
    );
    commands.entity(right_hand_target).insert(RightHandTarget);

    // Foot probes (invisible)
    commands.spawn((
        Name::new("L Foot Probe"),
        LeftFootProbe,
        Transform::from_xyz(-3.15, 0.0, 0.0),
        Visibility::default(),
    ));

    commands.spawn((
        Name::new("R Foot Probe"),
        RightFootProbe,
        Transform::from_xyz(-2.85, 0.0, 0.0),
        Visibility::default(),
    ));

    // --- IK Controllers ---

    // Left foot IK
    let left_foot_ctrl = commands
        .spawn((
            Name::new("L Foot Controller"),
            LeftFootController,
            IkDebugDraw::default(),
            IkChain {
                joints: joints.left_leg_chain(),
                solver: IkSolver::TwoBone,
                ..default()
            },
            FootPlacement {
                ankle_offset: 0.03,
                foot_up_axis: Vec3::Y,
                foot_forward_axis: Vec3::Z,
                root_offset_hint: Some(RootOffsetHint {
                    axis: Vec3::Y,
                    max_distance: 0.25,
                    weight: 1.0,
                }),
                ..default()
            },
            PoleTarget {
                point: Vec3::new(-3.15, 0.5, 1.0),
                ..default()
            },
        ))
        .id();

    // Right foot IK
    let right_foot_ctrl = commands
        .spawn((
            Name::new("R Foot Controller"),
            RightFootController,
            IkDebugDraw::default(),
            IkChain {
                joints: joints.right_leg_chain(),
                solver: IkSolver::TwoBone,
                ..default()
            },
            FootPlacement {
                ankle_offset: 0.03,
                foot_up_axis: Vec3::Y,
                foot_forward_axis: Vec3::Z,
                root_offset_hint: Some(RootOffsetHint {
                    axis: Vec3::Y,
                    max_distance: 0.25,
                    weight: 1.0,
                }),
                ..default()
            },
            PoleTarget {
                point: Vec3::new(-2.85, 0.5, 1.0),
                ..default()
            },
        ))
        .id();

    // Full body rig (coordinates both feet to move the pelvis)
    commands.spawn((
        Name::new("Full Body Rig"),
        FullBodyIkRig::new(joints.pelvis)
            .with_chain(left_foot_ctrl)
            .with_chain(right_foot_ctrl)
            .with_root_axis(Vec3::Y)
            .with_max_root_offset(0.25),
    ));

    // Left arm IK
    commands.spawn((
        Name::new("L Arm Controller"),
        LeftArmController,
        IkDebugDraw {
            color: Color::srgb(0.2, 0.9, 0.4),
            ..default()
        },
        IkChain {
            joints: joints.left_arm_chain(),
            solver: IkSolver::TwoBone,
            weight: IkWeight {
                overall: 1.0,
                position: 1.0,
                rotation: 0.5,
            },
            ..default()
        },
        IkTarget::default(),
        IkTargetAnchor {
            entity: left_hand_target,
            translation_offset: Vec3::ZERO,
            rotation_offset: Quat::IDENTITY,
        },
        PoleTarget {
            point: Vec3::new(-3.5, 0.5, -1.0),
            ..default()
        },
    ));

    // Right arm IK
    commands.spawn((
        Name::new("R Arm Controller"),
        RightArmController,
        IkDebugDraw {
            color: Color::srgb(0.9, 0.4, 0.2),
            ..default()
        },
        IkChain {
            joints: joints.right_arm_chain(),
            solver: IkSolver::TwoBone,
            weight: IkWeight {
                overall: 1.0,
                position: 1.0,
                rotation: 0.5,
            },
            ..default()
        },
        IkTarget::default(),
        IkTargetAnchor {
            entity: right_hand_target,
            translation_offset: Vec3::ZERO,
            rotation_offset: Quat::IDENTITY,
        },
        PoleTarget {
            point: Vec3::new(-2.5, 0.5, -1.0),
            ..default()
        },
    ));

    // Head look-at IK
    commands.spawn((
        Name::new("Look Controller"),
        LookController,
        IkDebugDraw {
            color: Color::srgb(0.95, 0.85, 0.3),
            ..default()
        },
        IkChain {
            joints: joints.look_chain(),
            ..default()
        },
        LookAtTarget {
            forward_axis: Vec3::Y,
            up_axis: Vec3::Z,
            ..default()
        },
        IkTargetAnchor {
            entity: look_target,
            translation_offset: Vec3::ZERO,
            rotation_offset: Quat::IDENTITY,
        },
    ));

    // Add cone constraints to neck/head for natural range
    commands.entity(joints.chest).insert(IkConstraint::Cone {
        axis: Vec3::Y,
        max_angle: 0.4,
        strength: 1.0,
    });
    commands.entity(joints.neck).insert(IkConstraint::Cone {
        axis: Vec3::Y,
        max_angle: 0.6,
        strength: 1.0,
    });

    // Overlay
    commands.spawn((
        Name::new("Overlay"),
        Overlay,
        Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            top: px(16.0),
            width: px(500.0),
            padding: UiRect::all(px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.85)),
        Text::new(String::new()),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

/// Animates the humanoid walking across terrain with foot probes sliding along the ground.
fn animate_walking(
    time: Res<Time>,
    mut mannequin: Single<&mut Transform, With<Mannequin>>,
    mut left_probe: Single<
        &mut Transform,
        (
            With<LeftFootProbe>,
            Without<Mannequin>,
            Without<RightFootProbe>,
            Without<LeftHandTarget>,
            Without<RightHandTarget>,
        ),
    >,
    mut right_probe: Single<
        &mut Transform,
        (
            With<RightFootProbe>,
            Without<Mannequin>,
            Without<LeftFootProbe>,
            Without<LeftHandTarget>,
            Without<RightHandTarget>,
        ),
    >,
    mut left_hand: Single<
        &mut Transform,
        (
            With<LeftHandTarget>,
            Without<Mannequin>,
            Without<LeftFootProbe>,
            Without<RightFootProbe>,
            Without<RightHandTarget>,
        ),
    >,
    mut right_hand: Single<
        &mut Transform,
        (
            With<RightHandTarget>,
            Without<Mannequin>,
            Without<LeftFootProbe>,
            Without<RightFootProbe>,
            Without<LeftHandTarget>,
        ),
    >,
    mut left_foot_ctrl: Single<
        (&mut FootPlacement, &mut PoleTarget),
        (With<LeftFootController>, Without<RightFootController>),
    >,
    mut right_foot_ctrl: Single<
        (&mut FootPlacement, &mut PoleTarget),
        (With<RightFootController>, Without<LeftFootController>),
    >,
) {
    let t = time.elapsed_secs() * 0.35;
    let hip_width = 0.36 * 0.35;

    // Character walks left-right across the terrain
    let walk_x = t.sin() * 3.2;
    mannequin.translation.x = walk_x;

    // Foot stride cycle
    let stride = 0.4;
    let step_height = 0.08;
    let cycle = t * 2.5;

    // Left foot (offset by half cycle from right)
    let left_phase = cycle;
    let left_stride_offset = left_phase.sin() * stride;
    let left_lift = (left_phase.sin().max(0.0)) * step_height;
    let left_x = walk_x - hip_width + left_stride_offset;
    let left_ground = height_at_x(left_x);
    left_probe.translation = Vec3::new(left_x, left_ground + left_lift, 0.0);
    left_foot_ctrl.0.contact_point = left_probe.translation;
    left_foot_ctrl.0.contact_normal = Vec3::Y;
    left_foot_ctrl.1.point = Vec3::new(left_x, left_ground + 0.5, 1.0);

    // Right foot (offset by half cycle)
    let right_phase = cycle + std::f32::consts::PI;
    let right_stride_offset = right_phase.sin() * stride;
    let right_lift = (right_phase.sin().max(0.0)) * step_height;
    let right_x = walk_x + hip_width + right_stride_offset;
    let right_ground = height_at_x(right_x);
    right_probe.translation = Vec3::new(right_x, right_ground + right_lift, 0.0);
    right_foot_ctrl.0.contact_point = right_probe.translation;
    right_foot_ctrl.0.contact_normal = Vec3::Y;
    right_foot_ctrl.1.point = Vec3::new(right_x, right_ground + 0.5, 1.0);

    // Arms swing opposite to legs
    let arm_swing = 0.3;
    left_hand.translation = Vec3::new(
        walk_x - hip_width * 2.0,
        0.55 + (-left_phase).sin().abs() * 0.1,
        (-left_phase).sin() * arm_swing,
    );
    right_hand.translation = Vec3::new(
        walk_x + hip_width * 2.0,
        0.55 + (-right_phase).sin().abs() * 0.1,
        (-right_phase).sin() * arm_swing,
    );
}

/// Updates the look-at target from the mouse cursor position in world space.
fn update_look_from_mouse(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut look_target: Single<&mut Transform, With<LookTarget>>,
    mut look_ctrl: Single<&mut LookAtTarget, With<LookController>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Cast a ray from cursor into the scene and find intersection at a plane
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    // Intersect with a vertical plane at Z=0 (or a horizontal plane at character head height)
    let plane_normal = Vec3::Z;
    let plane_point = Vec3::new(0.0, 1.0, 0.0);
    let denom = ray.direction.dot(plane_normal);
    if denom.abs() < 1e-6 {
        return;
    }
    let t = (plane_point - ray.origin).dot(plane_normal) / denom;
    if t < 0.0 {
        return;
    }

    let hit = ray.origin + *ray.direction * t;
    look_target.translation = hit;
    look_ctrl.point = hit;
}

fn sync_humanoid_pane(
    mut pane: ResMut<HumanoidPane>,
    mut debug: ResMut<IkDebugSettings>,
    mut left_foot: Single<
        (&mut IkChain, &mut FootPlacement, &IkChainState),
        (
            With<LeftFootController>,
            Without<RightFootController>,
            Without<LeftArmController>,
            Without<RightArmController>,
            Without<LookController>,
        ),
    >,
    mut right_foot: Single<
        (&mut IkChain, &mut FootPlacement, &IkChainState),
        (
            With<RightFootController>,
            Without<LeftFootController>,
            Without<LeftArmController>,
            Without<RightArmController>,
            Without<LookController>,
        ),
    >,
    mut left_arm: Single<
        &mut IkChain,
        (
            With<LeftArmController>,
            Without<RightArmController>,
            Without<LeftFootController>,
            Without<RightFootController>,
            Without<LookController>,
        ),
    >,
    mut right_arm: Single<
        &mut IkChain,
        (
            With<RightArmController>,
            Without<LeftArmController>,
            Without<LeftFootController>,
            Without<RightFootController>,
            Without<LookController>,
        ),
    >,
    mut look: Single<
        (&mut IkChain, &IkChainState),
        (
            With<LookController>,
            Without<LeftFootController>,
            Without<RightFootController>,
            Without<LeftArmController>,
            Without<RightArmController>,
        ),
    >,
    mut rig: Single<&mut FullBodyIkRig>,
) {
    if pane.is_changed() && !pane.is_added() {
        debug.enabled = pane.debug_enabled;
        left_foot.0.solve.iterations = pane.iterations;
        right_foot.0.solve.iterations = pane.iterations;
        left_arm.solve.iterations = pane.iterations;
        right_arm.solve.iterations = pane.iterations;
        look.0.solve.iterations = pane.iterations;

        left_foot.1.ankle_offset = pane.ankle_offset;
        right_foot.1.ankle_offset = pane.ankle_offset;
        rig.max_root_offset = pane.max_root_offset;

        left_arm.weight.overall = pane.arm_weight;
        right_arm.weight.overall = pane.arm_weight;

        look.0.weight.overall = pane.look_weight;
    }

    let pane = pane.bypass_change_detection();
    pane.debug_enabled = debug.enabled;
    pane.iterations = left_foot.0.solve.iterations;
    pane.ankle_offset = left_foot.1.ankle_offset;
    pane.max_root_offset = rig.max_root_offset;
    pane.arm_weight = left_arm.weight.overall;
    pane.look_weight = look.0.weight.overall;
    pane.left_foot_error = left_foot.2.last_error;
    pane.right_foot_error = right_foot.2.last_error;
    pane.look_error = look.1.last_error;
}

fn update_overlay(
    pane: Res<HumanoidPane>,
    rig_state: Single<&FullBodyIkRigState>,
    mut overlay: Single<&mut Text, With<Overlay>>,
) {
    let mut text = String::new();
    let _ = writeln!(text, "HUMANOID IK DEMO");
    let _ = writeln!(text);
    let _ = writeln!(text, "Move mouse to control head look-at direction");
    let _ = writeln!(text, "Character walks automatically across terrain");
    let _ = writeln!(text);
    let _ = writeln!(
        text,
        "feet error: L {:.3}  R {:.3}",
        pane.left_foot_error, pane.right_foot_error
    );
    let _ = writeln!(
        text,
        "pelvis offset: ({:.2}, {:.2}, {:.2})",
        rig_state.combined_root_offset.x,
        rig_state.combined_root_offset.y,
        rig_state.combined_root_offset.z,
    );
    let _ = writeln!(text, "look error: {:.3}", pane.look_error);
    let _ = writeln!(text, "debug: {}", pane.debug_enabled);
    overlay.0 = text;
}
