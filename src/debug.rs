use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::{
    components::{IkChain, IkDebugDraw},
    constraints::IkConstraint,
    math::{project_on_plane, safe_normalize},
    systems::{IkPreparedChain, IkSolvedChain},
};

#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct IkDebugSettings {
    pub enabled: bool,
    pub draw_targets: bool,
    pub draw_pole_vectors: bool,
    pub draw_reach_radius: bool,
    pub draw_error_lines: bool,
    pub draw_constraints: bool,
}

impl Default for IkDebugSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            draw_targets: true,
            draw_pole_vectors: true,
            draw_reach_radius: true,
            draw_error_lines: true,
            draw_constraints: true,
        }
    }
}

pub(crate) fn draw_debug_gizmos(
    debug: Res<IkDebugSettings>,
    chains: Query<(
        &IkChain,
        Option<&IkDebugDraw>,
        Option<&IkPreparedChain>,
        Option<&IkSolvedChain>,
    )>,
    mut gizmos: Gizmos,
) {
    if !debug.enabled {
        return;
    }

    for (chain, chain_debug, prepared, solved) in &chains {
        if !chain.enabled {
            continue;
        }

        let draw = chain_debug.cloned().unwrap_or_default();
        if !draw.enabled {
            continue;
        }

        let Some(prepared) = prepared else {
            continue;
        };

        let positions = solved
            .map(|solved| solved.positions.as_slice())
            .unwrap_or(prepared.positions.as_slice());
        let color = draw.color;

        for joint in positions {
            gizmos.sphere(*joint, draw.joint_radius, color);
        }

        gizmos.linestrip(positions.iter().copied(), color);

        if debug.draw_targets {
            gizmos.sphere(
                prepared.target.position,
                draw.joint_radius * 1.35,
                Color::srgb(1.0, 0.3, 0.2),
            );
        }

        if debug.draw_reach_radius {
            gizmos.circle(
                Isometry3d::from_translation(prepared.positions[0]),
                prepared.total_length,
                Color::srgba(0.5, 0.9, 1.0, 0.35),
            );
        }

        if debug.draw_pole_vectors {
            if let Some(pole) = prepared.pole {
                gizmos.line(
                    prepared.positions[0],
                    pole.point,
                    Color::srgb(0.94, 0.78, 0.28),
                );
                gizmos.sphere(pole.point, draw.joint_radius, Color::srgb(0.94, 0.78, 0.28));
            }
        }

        if debug.draw_error_lines {
            if let Some(effector) = positions.last().copied() {
                gizmos.line(
                    effector,
                    prepared.target.position,
                    Color::srgb(1.0, 0.2, 0.4),
                );
            }
        }

        if debug.draw_constraints && draw.draw_constraints {
            for (index, joint) in prepared
                .joints
                .iter()
                .enumerate()
                .take(prepared.constraints.len())
            {
                let Some(constraint) = &prepared.constraints[index] else {
                    continue;
                };
                let position = positions[index];
                draw_constraint(
                    &mut gizmos,
                    position,
                    joint.authored_rotation,
                    joint.settings.tip_axis,
                    constraint,
                    color,
                    draw.joint_radius * 10.0,
                );
            }
        }
    }
}

fn draw_constraint(
    gizmos: &mut Gizmos,
    origin: Vec3,
    authored_rotation: Quat,
    tip_axis_local: Vec3,
    constraint: &IkConstraint,
    color: Color,
    scale: f32,
) {
    match constraint {
        IkConstraint::Cone {
            axis, max_angle, ..
        } => {
            let axis_world = authored_rotation * safe_normalize(*axis, tip_axis_local);
            gizmos.line(origin, origin + axis_world * scale, color);

            let tangent = axis_world.any_orthonormal_vector();
            let bitangent = axis_world.cross(tangent).normalize_or_zero();
            let cone_center = origin + axis_world * (scale * max_angle.cos());
            let radius = scale * max_angle.sin();
            let mut points = Vec::with_capacity(25);
            for step in 0..=24 {
                let t = TAU * step as f32 / 24.0;
                let ring = tangent * (radius * t.cos()) + bitangent * (radius * t.sin());
                points.push(cone_center + ring);
            }
            gizmos.linestrip(points, color);
        }
        IkConstraint::Hinge {
            axis,
            reference_axis,
            min_angle,
            max_angle,
            ..
        } => {
            let hinge_axis = authored_rotation * safe_normalize(*axis, Vec3::Z);
            let reference = authored_rotation * safe_normalize(*reference_axis, tip_axis_local);
            let planar_reference = safe_normalize(
                project_on_plane(reference, hinge_axis),
                hinge_axis.any_orthonormal_vector(),
            );
            gizmos.line(origin, origin + hinge_axis * scale, color);

            let mut points = Vec::with_capacity(25);
            for step in 0..=24 {
                let t = step as f32 / 24.0;
                let angle = min_angle + (max_angle - min_angle) * t;
                let dir = Quat::from_axis_angle(hinge_axis, angle) * planar_reference;
                points.push(origin + dir * scale);
            }
            gizmos.linestrip(points, color);
        }
    }
}
