use bevy::prelude::*;
use saddle_animation_ik::IkJoint;

/// Joint entity handles returned after spawning a capsule humanoid.
/// Each field holds the entity IDs needed to build IK chains.
pub struct HumanoidJoints {
    pub root: Entity,
    pub pelvis: Entity,
    pub spine: Entity,
    pub chest: Entity,
    pub neck: Entity,
    pub head: Entity,
    pub left_upper_arm: Entity,
    pub left_lower_arm: Entity,
    pub left_hand: Entity,
    pub right_upper_arm: Entity,
    pub right_lower_arm: Entity,
    pub right_hand: Entity,
    pub left_upper_leg: Entity,
    pub left_lower_leg: Entity,
    pub left_foot: Entity,
    pub right_upper_leg: Entity,
    pub right_lower_leg: Entity,
    pub right_foot: Entity,
}

impl HumanoidJoints {
    /// Left arm chain: shoulder -> elbow -> wrist (two-bone IK).
    pub fn left_arm_chain(&self) -> Vec<Entity> {
        vec![self.left_upper_arm, self.left_lower_arm, self.left_hand]
    }

    /// Right arm chain: shoulder -> elbow -> wrist (two-bone IK).
    pub fn right_arm_chain(&self) -> Vec<Entity> {
        vec![self.right_upper_arm, self.right_lower_arm, self.right_hand]
    }

    /// Left leg chain: hip -> knee -> ankle (two-bone IK).
    pub fn left_leg_chain(&self) -> Vec<Entity> {
        vec![self.left_upper_leg, self.left_lower_leg, self.left_foot]
    }

    /// Right leg chain: hip -> knee -> ankle (two-bone IK).
    pub fn right_leg_chain(&self) -> Vec<Entity> {
        vec![self.right_upper_leg, self.right_lower_leg, self.right_foot]
    }

    /// Head/neck look chain: chest -> neck -> head (FABRIK with cone constraints).
    pub fn look_chain(&self) -> Vec<Entity> {
        vec![self.chest, self.neck, self.head]
    }
}

/// Body proportions for the capsule humanoid.
pub struct HumanoidProportions {
    /// Total height from ground to top of head.
    pub height: f32,
    /// Width of the torso.
    pub torso_width: f32,
    /// Radius of capsule limbs.
    pub limb_radius: f32,
}

impl Default for HumanoidProportions {
    fn default() -> Self {
        Self {
            height: 1.8,
            torso_width: 0.36,
            limb_radius: 0.04,
        }
    }
}

/// Spawns a capsule humanoid made from primitive meshes with proper bone hierarchy.
///
/// The humanoid stands at `position` and faces along +Z. All joints have `IkJoint`
/// components configured for their anatomical role.
///
/// Returns `HumanoidJoints` with entity handles for building IK chains.
pub fn spawn_capsule_humanoid(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    position: Vec3,
    proportions: &HumanoidProportions,
    body_color: Color,
    joint_color: Color,
) -> HumanoidJoints {
    let h = proportions.height;
    let r = proportions.limb_radius;
    let tw = proportions.torso_width;

    // Derived measurements
    let upper_leg_len = h * 0.245;
    let lower_leg_len = h * 0.245;
    let upper_arm_len = h * 0.17;
    let lower_arm_len = h * 0.155;
    let torso_len = h * 0.19;
    let neck_len = h * 0.06;
    let head_radius = h * 0.06;
    let hand_radius = r * 1.2;
    let foot_len = h * 0.08;
    let shoulder_offset_y = h * 0.01;

    let body_mat = materials.add(StandardMaterial {
        base_color: body_color,
        perceptual_roughness: 0.85,
        ..default()
    });
    let joint_mat = materials.add(StandardMaterial {
        base_color: joint_color,
        emissive: joint_color.into(),
        perceptual_roughness: 0.7,
        ..default()
    });

    let joint_sphere = meshes.add(Sphere::new(r * 2.0).mesh().uv(12, 8));
    let head_mesh = meshes.add(Sphere::new(head_radius).mesh().uv(16, 12));
    let hand_mesh = meshes.add(Sphere::new(hand_radius).mesh().uv(10, 8));

    fn capsule_mesh(meshes: &mut Assets<Mesh>, half_length: f32, radius: f32) -> Handle<Mesh> {
        meshes.add(Capsule3d::new(radius, half_length * 2.0).mesh())
    }

    let torso_mesh = capsule_mesh(meshes, torso_len * 0.5, tw * 0.5);
    let upper_leg_mesh = capsule_mesh(meshes, upper_leg_len * 0.5, r * 1.5);
    let lower_leg_mesh = capsule_mesh(meshes, lower_leg_len * 0.5, r * 1.3);
    let upper_arm_mesh = capsule_mesh(meshes, upper_arm_len * 0.5, r * 1.3);
    let lower_arm_mesh = capsule_mesh(meshes, lower_arm_len * 0.5, r * 1.1);
    let foot_mesh = meshes.add(Cuboid::new(r * 3.0, r * 1.5, foot_len));

    // IkJoint configs per body part role
    let leg_joint = IkJoint {
        tip_axis: -Vec3::Y,
        pole_axis: Vec3::Z,
        ..default()
    };
    let arm_joint_left = IkJoint {
        tip_axis: -Vec3::X,
        pole_axis: -Vec3::Z,
        ..default()
    };
    let arm_joint_right = IkJoint {
        tip_axis: Vec3::X,
        pole_axis: -Vec3::Z,
        ..default()
    };
    let spine_joint = IkJoint {
        tip_axis: Vec3::Y,
        pole_axis: Vec3::Z,
        ..default()
    };

    // -- Root (invisible, at ground level) --
    let root = commands
        .spawn((
            Name::new(format!("{name} Root")),
            Transform::from_translation(position),
            Visibility::default(),
        ))
        .id();

    // -- Pelvis (at hip height) --
    let pelvis_y = upper_leg_len + lower_leg_len;
    let pelvis = commands
        .spawn((
            Name::new(format!("{name} Pelvis")),
            Mesh3d(joint_sphere.clone()),
            MeshMaterial3d(joint_mat.clone()),
            Transform::from_xyz(0.0, pelvis_y, 0.0),
            spine_joint,
        ))
        .id();
    commands.entity(root).add_child(pelvis);

    // -- Spine (between pelvis and chest) --
    let spine = commands
        .spawn((
            Name::new(format!("{name} Spine")),
            Mesh3d(torso_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, torso_len * 0.5, 0.0),
            spine_joint,
        ))
        .id();
    commands.entity(pelvis).add_child(spine);

    // -- Chest (top of torso) --
    let chest = commands
        .spawn((
            Name::new(format!("{name} Chest")),
            Mesh3d(joint_sphere.clone()),
            MeshMaterial3d(joint_mat.clone()),
            Transform::from_xyz(0.0, torso_len * 0.5, 0.0),
            spine_joint,
        ))
        .id();
    commands.entity(spine).add_child(chest);

    // -- Neck --
    let neck = commands
        .spawn((
            Name::new(format!("{name} Neck")),
            Mesh3d(joint_sphere.clone()),
            MeshMaterial3d(joint_mat.clone()),
            Transform::from_xyz(0.0, neck_len, 0.0),
            spine_joint,
        ))
        .id();
    commands.entity(chest).add_child(neck);

    // -- Head --
    let head = commands
        .spawn((
            Name::new(format!("{name} Head")),
            Mesh3d(head_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, head_radius + r, 0.0),
            spine_joint,
        ))
        .id();
    commands.entity(neck).add_child(head);

    // -- Left Arm --
    let left_upper_arm = commands
        .spawn((
            Name::new(format!("{name} L Upper Arm")),
            Mesh3d(upper_arm_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(-(tw * 0.5 + r), shoulder_offset_y, 0.0)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            arm_joint_left,
        ))
        .id();
    commands.entity(chest).add_child(left_upper_arm);

    let left_lower_arm = commands
        .spawn((
            Name::new(format!("{name} L Lower Arm")),
            Mesh3d(lower_arm_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(-upper_arm_len, 0.0, 0.0),
            arm_joint_left,
        ))
        .id();
    commands.entity(left_upper_arm).add_child(left_lower_arm);

    let left_hand = commands
        .spawn((
            Name::new(format!("{name} L Hand")),
            Mesh3d(hand_mesh.clone()),
            MeshMaterial3d(joint_mat.clone()),
            Transform::from_xyz(-lower_arm_len, 0.0, 0.0),
            arm_joint_left,
        ))
        .id();
    commands.entity(left_lower_arm).add_child(left_hand);

    // -- Right Arm --
    let right_upper_arm = commands
        .spawn((
            Name::new(format!("{name} R Upper Arm")),
            Mesh3d(upper_arm_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(tw * 0.5 + r, shoulder_offset_y, 0.0)
                .with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
            arm_joint_right,
        ))
        .id();
    commands.entity(chest).add_child(right_upper_arm);

    let right_lower_arm = commands
        .spawn((
            Name::new(format!("{name} R Lower Arm")),
            Mesh3d(lower_arm_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(lower_arm_len, 0.0, 0.0),
            arm_joint_right,
        ))
        .id();
    commands.entity(right_upper_arm).add_child(right_lower_arm);

    let right_hand = commands
        .spawn((
            Name::new(format!("{name} R Hand")),
            Mesh3d(hand_mesh),
            MeshMaterial3d(joint_mat.clone()),
            Transform::from_xyz(lower_arm_len, 0.0, 0.0),
            arm_joint_right,
        ))
        .id();
    commands.entity(right_lower_arm).add_child(right_hand);

    // -- Left Leg --
    let left_upper_leg = commands
        .spawn((
            Name::new(format!("{name} L Upper Leg")),
            Mesh3d(upper_leg_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(-(tw * 0.35), 0.0, 0.0),
            leg_joint,
        ))
        .id();
    commands.entity(pelvis).add_child(left_upper_leg);

    let left_lower_leg = commands
        .spawn((
            Name::new(format!("{name} L Lower Leg")),
            Mesh3d(lower_leg_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, -upper_leg_len, 0.0),
            leg_joint,
        ))
        .id();
    commands.entity(left_upper_leg).add_child(left_lower_leg);

    let left_foot = commands
        .spawn((
            Name::new(format!("{name} L Foot")),
            Mesh3d(foot_mesh.clone()),
            MeshMaterial3d(joint_mat.clone()),
            Transform::from_xyz(0.0, -lower_leg_len, foot_len * 0.3),
            leg_joint,
        ))
        .id();
    commands.entity(left_lower_leg).add_child(left_foot);

    // -- Right Leg --
    let right_upper_leg = commands
        .spawn((
            Name::new(format!("{name} R Upper Leg")),
            Mesh3d(upper_leg_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(tw * 0.35, 0.0, 0.0),
            leg_joint,
        ))
        .id();
    commands.entity(pelvis).add_child(right_upper_leg);

    let right_lower_leg = commands
        .spawn((
            Name::new(format!("{name} R Lower Leg")),
            Mesh3d(lower_leg_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, -upper_leg_len, 0.0),
            leg_joint,
        ))
        .id();
    commands.entity(right_upper_leg).add_child(right_lower_leg);

    let right_foot = commands
        .spawn((
            Name::new(format!("{name} R Foot")),
            Mesh3d(foot_mesh),
            MeshMaterial3d(joint_mat),
            Transform::from_xyz(0.0, -lower_leg_len, foot_len * 0.3),
            leg_joint,
        ))
        .id();
    commands.entity(right_lower_leg).add_child(right_foot);

    HumanoidJoints {
        root,
        pelvis,
        spine,
        chest,
        neck,
        head,
        left_upper_arm,
        left_lower_arm,
        left_hand,
        right_upper_arm,
        right_lower_arm,
        right_hand,
        left_upper_leg,
        left_lower_leg,
        left_foot,
        right_upper_leg,
        right_lower_leg,
        right_foot,
    }
}
