use bevy::prelude::*;

use crate::math::{project_on_plane, safe_normalize, signed_angle_on_axis};

#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub enum IkConstraint {
    Cone {
        axis: Vec3,
        max_angle: f32,
        strength: f32,
    },
    Hinge {
        axis: Vec3,
        reference_axis: Vec3,
        min_angle: f32,
        max_angle: f32,
        strength: f32,
    },
}

impl IkConstraint {
    pub fn constrain(
        &self,
        direction_world: Vec3,
        authored_rotation: Quat,
        tip_axis_local: Vec3,
    ) -> Vec3 {
        let fallback = authored_rotation * safe_normalize(tip_axis_local, Vec3::Y);
        let desired = safe_normalize(direction_world, fallback);

        match self {
            Self::Cone {
                axis,
                max_angle,
                strength,
            } => {
                let axis_world = authored_rotation * safe_normalize(*axis, tip_axis_local);
                constrain_cone(desired, axis_world, *max_angle, *strength)
            }
            Self::Hinge {
                axis,
                reference_axis,
                min_angle,
                max_angle,
                strength,
            } => {
                let axis_world = authored_rotation * safe_normalize(*axis, Vec3::Z);
                let reference_world =
                    authored_rotation * safe_normalize(*reference_axis, tip_axis_local);
                constrain_hinge(
                    desired,
                    axis_world,
                    reference_world,
                    *min_angle,
                    *max_angle,
                    *strength,
                )
            }
        }
    }
}

fn constrain_cone(direction: Vec3, axis: Vec3, max_angle: f32, strength: f32) -> Vec3 {
    let axis = safe_normalize(axis, Vec3::Y);
    let max_angle = max_angle.max(0.0);
    let strength = strength.clamp(0.0, 1.0);
    let current_angle = direction.angle_between(axis);
    if current_angle <= max_angle {
        return direction;
    }

    let tangent = safe_normalize(
        direction - axis * direction.dot(axis),
        axis.any_orthonormal_vector(),
    );
    let constrained = (axis * max_angle.cos() + tangent * max_angle.sin()).normalize_or_zero();
    direction.slerp(safe_normalize(constrained, axis), strength)
}

fn constrain_hinge(
    direction: Vec3,
    hinge_axis: Vec3,
    reference_axis: Vec3,
    min_angle: f32,
    max_angle: f32,
    strength: f32,
) -> Vec3 {
    let hinge_axis = safe_normalize(hinge_axis, Vec3::Z);
    let reference_axis = safe_normalize(
        project_on_plane(reference_axis, hinge_axis),
        hinge_axis.any_orthonormal_vector(),
    );
    let desired_on_plane = safe_normalize(project_on_plane(direction, hinge_axis), reference_axis);
    let clamped_angle = signed_angle_on_axis(reference_axis, desired_on_plane, hinge_axis)
        .clamp(min_angle, max_angle);
    let constrained = Quat::from_axis_angle(hinge_axis, clamped_angle) * reference_axis;
    desired_on_plane.slerp(constrained, strength.clamp(0.0, 1.0))
}
