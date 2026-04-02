use bevy::prelude::*;

pub fn safe_normalize(vector: Vec3, fallback: Vec3) -> Vec3 {
    vector
        .try_normalize()
        .unwrap_or_else(|| fallback.try_normalize().unwrap_or(Vec3::Y))
}

pub fn project_on_plane(vector: Vec3, plane_normal: Vec3) -> Vec3 {
    let normal = safe_normalize(plane_normal, Vec3::Y);
    vector - normal * vector.dot(normal)
}

pub fn signed_angle_on_axis(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
    let axis = safe_normalize(axis, Vec3::Y);
    let from = safe_normalize(project_on_plane(from, axis), axis.any_orthonormal_vector());
    let to = safe_normalize(project_on_plane(to, axis), from);
    let angle = from.angle_between(to);
    let sign = axis.dot(from.cross(to)).signum();
    angle * if sign == 0.0 { 1.0 } else { sign }
}

pub fn align_axis_rotation(
    base_rotation: Quat,
    local_axis: Vec3,
    desired_world_axis: Vec3,
) -> Quat {
    let current_world_axis = base_rotation * safe_normalize(local_axis, Vec3::Y);
    let desired_world_axis = safe_normalize(desired_world_axis, current_world_axis);

    let dot = current_world_axis.dot(desired_world_axis);
    if dot > 0.999_99 {
        return base_rotation;
    }

    if dot < -0.999_99 {
        let fallback_axis = project_on_plane(Vec3::X, current_world_axis);
        let rotation_axis =
            safe_normalize(fallback_axis, current_world_axis.any_orthonormal_vector());
        return Quat::from_axis_angle(rotation_axis, std::f32::consts::PI) * base_rotation;
    }

    Quat::from_rotation_arc(current_world_axis, desired_world_axis) * base_rotation
}

pub fn orientation_from_axes(
    local_forward: Vec3,
    local_up: Vec3,
    world_forward: Vec3,
    world_up: Vec3,
) -> Quat {
    let local_forward = safe_normalize(local_forward, Vec3::Z);
    let local_up = safe_normalize(project_on_plane(local_up, local_forward), Vec3::Y);
    let local_right = safe_normalize(local_up.cross(local_forward), Vec3::X);
    let local_basis = Mat3::from_cols(local_right, local_up, local_forward);

    let world_forward = safe_normalize(world_forward, Vec3::Z);
    let world_up = safe_normalize(project_on_plane(world_up, world_forward), Vec3::Y);
    let world_right = safe_normalize(world_up.cross(world_forward), Vec3::X);
    let world_basis = Mat3::from_cols(world_right, world_up, world_forward);

    Quat::from_mat3(&world_basis) * Quat::from_mat3(&local_basis).inverse()
}

pub fn compute_root_offset_hint(
    root_position: Vec3,
    target_position: Vec3,
    total_length: f32,
    axis: Vec3,
    max_distance: f32,
    weight: f32,
) -> Vec3 {
    let axis = safe_normalize(axis, Vec3::Y);
    let delta = target_position - root_position;
    let along_axis = delta.dot(axis);
    let planar = project_on_plane(delta, axis);
    let planar_length = planar.length();
    let max_axis_distance = (total_length * total_length - planar_length * planar_length)
        .max(0.0)
        .sqrt();

    let overflow = along_axis.abs() - max_axis_distance;
    if overflow <= 0.0 {
        return Vec3::ZERO;
    }

    axis * along_axis.signum() * overflow.min(max_distance.max(0.0)) * weight.clamp(0.0, 1.0)
}
