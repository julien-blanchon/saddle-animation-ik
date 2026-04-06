use saddle_animation_ik_example_support as support;

use std::fmt::Write as _;

use bevy::prelude::*;
use saddle_animation_ik::{
    IkChain, IkDebugSettings, IkPlugin, IkTarget, IkTargetAnchor, PoleTarget,
};
use saddle_pane::prelude::*;
use support::{OrbitMotion, animate_orbits, spawn_joint_chain, spawn_target};

#[derive(Component)]
struct BasicReachController;

#[derive(Component)]
struct ReachTarget;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Overlay;

#[derive(Resource)]
struct MouseMode(bool);

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
                    title: "ik basic reach — press M to toggle mouse/orbit mode".into(),
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
        .insert_resource(MouseMode(false))
        .init_resource::<BasicPane>()
        .register_pane::<BasicPane>()
        .add_plugins(IkPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_mouse_mode,
                animate_orbits.run_if(|mode: Res<MouseMode>| !mode.0),
                move_target_mouse.run_if(|mode: Res<MouseMode>| mode.0),
                sync_basic_pane,
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
    commands.spawn((
        Name::new("Demo Camera"),
        MainCamera,
        Camera3d::default(),
        Transform::from_xyz(-7.0, 5.0, 10.0).looking_at(Vec3::new(-2.0, 1.5, 0.0), Vec3::Y),
    ));

    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: 19_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 18.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(30.0, 30.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.13, 0.15),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    let target = spawn_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Reach Target",
        Vec3::new(1.0, 2.0, 0.0),
        Color::srgb(1.0, 0.32, 0.2),
    );
    commands.entity(target).insert((
        ReachTarget,
        OrbitMotion {
            center: Vec3::new(-1.0, 2.2, 0.0),
            radius: Vec3::new(1.8, 0.7, 1.1),
            speed: 0.9,
            phase: 0.0,
        },
    ));

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

    // Overlay
    commands.spawn((
        Name::new("Overlay"),
        Overlay,
        Node {
            position_type: PositionType::Absolute,
            left: px(14.0),
            top: px(14.0),
            width: px(380.0),
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

fn toggle_mouse_mode(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<MouseMode>) {
    if keys.just_pressed(KeyCode::KeyM) {
        mode.0 = !mode.0;
    }
}

fn move_target_mouse(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut target: Single<&mut Transform, With<ReachTarget>>,
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
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    // Intersect with vertical plane at Z=0
    let denom = ray.direction.dot(Vec3::Z);
    if denom.abs() < 1e-6 {
        return;
    }
    let t = -ray.origin.z / denom;
    if t < 0.0 {
        return;
    }
    let hit = ray.origin + *ray.direction * t;
    target.translation = hit;
}

fn sync_basic_pane(
    mut pane: ResMut<BasicPane>,
    mut debug: ResMut<IkDebugSettings>,
    mut controller: Single<
        (&mut IkChain, &saddle_animation_ik::IkChainState),
        With<BasicReachController>,
    >,
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

fn update_overlay(
    mode: Res<MouseMode>,
    pane: Res<BasicPane>,
    mut overlay: Single<&mut Text, With<Overlay>>,
) {
    let mut text = String::new();
    let _ = writeln!(text, "BASIC REACH IK");
    let mode_str = if mode.0 { "MOUSE" } else { "ORBIT" };
    let _ = writeln!(text, "mode: {} (press M to toggle)", mode_str);
    let _ = writeln!(text, "error: {:.3}", pane.reach_error);
    overlay.0 = text;
}
