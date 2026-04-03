use bevy::prelude::*;
use saddle_animation_ik::IkJoint;

#[derive(Component, Clone, Copy)]
pub struct OrbitMotion {
    pub center: Vec3,
    pub radius: Vec3,
    pub speed: f32,
    pub phase: f32,
}

pub fn pane_plugins() -> (
    bevy_flair::FlairPlugin,
    bevy_input_focus::InputDispatchPlugin,
    bevy_ui_widgets::UiWidgetsPlugins,
    bevy_input_focus::tab_navigation::TabNavigationPlugin,
    saddle_pane::PanePlugin,
) {
    (
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    )
}

pub fn setup_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    camera_transform: Transform,
) {
    commands.spawn((
        Name::new("Demo Camera"),
        Camera3d::default(),
        camera_transform,
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
}

pub fn spawn_joint_chain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    root_transform: Transform,
    segment_lengths: &[f32],
    direction_local: Vec3,
    color: Color,
    joint: IkJoint,
) -> Vec<Entity> {
    let mut entities = Vec::with_capacity(segment_lengths.len() + 1);
    let sphere = meshes.add(Sphere::new(0.12).mesh().uv(20, 12));
    let material = materials.add(StandardMaterial {
        base_color: color,
        emissive: color.into(),
        perceptual_roughness: 0.85,
        ..default()
    });

    let root = commands
        .spawn((
            Name::new(format!("{name} Root")),
            Mesh3d(sphere.clone()),
            MeshMaterial3d(material.clone()),
            root_transform,
            joint,
        ))
        .id();
    entities.push(root);

    let mut parent = root;
    let direction_local = direction_local.normalize_or_zero();
    for (index, length) in segment_lengths.iter().copied().enumerate() {
        let child = commands
            .spawn((
                Name::new(format!("{name} Joint {}", index + 1)),
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(direction_local * length),
                joint,
            ))
            .id();
        commands.entity(parent).add_child(child);
        entities.push(child);
        parent = child;
    }

    entities
}

pub fn spawn_target(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    position: Vec3,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Name::new(name.to_string()),
            Mesh3d(meshes.add(Sphere::new(0.18).mesh().uv(20, 12))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: color.into(),
                ..default()
            })),
            Transform::from_translation(position),
        ))
        .id()
}

pub fn animate_orbits(time: Res<Time>, mut movers: Query<(&OrbitMotion, &mut Transform)>) {
    for (motion, mut transform) in &mut movers {
        let t = time.elapsed_secs() * motion.speed + motion.phase;
        transform.translation = motion.center
            + Vec3::new(
                motion.radius.x * t.cos(),
                motion.radius.y * (t * 1.3).sin(),
                motion.radius.z * t.sin(),
            );
    }
}
