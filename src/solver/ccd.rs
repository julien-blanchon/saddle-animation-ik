use bevy::prelude::*;

use crate::{
    config::{IkConstraintEnforcement, IkSolveSettings, IkSolveStatus},
    math::safe_normalize,
};

use super::{
    ResolvedPole, SolveResult, SolverChainState, apply_pole_target, reconstruct_with_constraints,
};

pub fn solve_ccd(
    chain: &SolverChainState,
    target: Vec3,
    pole: Option<ResolvedPole>,
    settings: IkSolveSettings,
) -> SolveResult {
    let mut positions = chain.positions();
    let mut status = IkSolveStatus::ReachedLimit;

    for _ in 0..settings.iterations {
        for joint_index in (0..positions.len() - 1).rev() {
            let joint_position = positions[joint_index];
            let effector_position = *positions.last().unwrap_or(&joint_position);
            let to_effector = safe_normalize(effector_position - joint_position, Vec3::Y);
            let to_target = safe_normalize(target - joint_position, to_effector);

            let dot = to_effector.dot(to_target);
            let rotation = if dot < -0.999_99 {
                Quat::from_axis_angle(to_effector.any_orthonormal_vector(), std::f32::consts::PI)
            } else if dot > 0.999_99 {
                Quat::IDENTITY
            } else {
                Quat::from_rotation_arc(to_effector, to_target)
            };

            for position in positions.iter_mut().skip(joint_index + 1) {
                let local = *position - joint_position;
                *position = joint_position + rotation * local;
            }
        }

        if let Some(pole) = pole {
            apply_pole_target(&mut positions, pole);
        }

        if matches!(
            settings.constraint_enforcement,
            IkConstraintEnforcement::AfterEachIteration
        ) {
            positions = reconstruct_with_constraints(chain, &positions);
        }

        let error = positions
            .last()
            .copied()
            .unwrap_or(Vec3::ZERO)
            .distance(target);
        if error <= settings.tolerance {
            status = IkSolveStatus::Solved;
            break;
        }
    }

    if matches!(
        settings.constraint_enforcement,
        IkConstraintEnforcement::AfterSolve
    ) {
        positions = reconstruct_with_constraints(chain, &positions);
    }

    let error = positions
        .last()
        .copied()
        .unwrap_or(Vec3::ZERO)
        .distance(target);
    SolveResult {
        positions,
        rotations: Vec::new(),
        error,
        unreachable: false,
        status,
    }
}
