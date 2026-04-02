use bevy::prelude::*;

use crate::{
    config::{IkConstraintEnforcement, IkSolveSettings, IkSolveStatus},
    math::{project_on_plane, safe_normalize},
};

use super::{
    ResolvedPole, SolveResult, SolverChainState, apply_pole_target, reconstruct_with_constraints,
};

pub fn solve_two_bone(
    chain: &SolverChainState,
    target: Vec3,
    pole: Option<ResolvedPole>,
    settings: IkSolveSettings,
) -> SolveResult {
    if chain.joints.len() != 3 || chain.lengths.len() != 2 {
        return SolveResult::invalid(chain);
    }

    let root = chain.joints[0].position;
    let current_mid = chain.joints[1].position;
    let authored_pole = safe_normalize(
        project_on_plane(current_mid - root, safe_normalize(target - root, Vec3::Y)),
        Vec3::Z,
    );

    let to_target = target - root;
    let distance = to_target.length();
    let direction = safe_normalize(to_target, Vec3::Y);
    let l1 = chain.lengths[0];
    let l2 = chain.lengths[1];
    let max_reach = l1 + l2;
    let clamped_distance = distance.clamp((l1 - l2).abs() + 0.0001, max_reach);
    let unreachable = distance >= max_reach;

    let pole_direction = pole
        .map(|pole| {
            safe_normalize(
                project_on_plane(pole.point - root, direction),
                authored_pole,
            )
        })
        .unwrap_or(authored_pole);

    let bend_normal = safe_normalize(
        direction.cross(pole_direction),
        direction.any_orthonormal_vector(),
    );
    let bend_axis = safe_normalize(bend_normal.cross(direction), pole_direction);

    let cos_angle = ((l1 * l1 + clamped_distance * clamped_distance - l2 * l2)
        / (2.0 * l1 * clamped_distance))
        .clamp(-1.0, 1.0);
    let along = l1 * cos_angle;
    let height = (l1 * l1 - along * along).max(0.0).sqrt();

    let mid = root + direction * along + bend_axis * height;
    let effector = root + direction * clamped_distance;
    let mut positions = vec![root, mid, effector];

    if let Some(pole) = pole {
        apply_pole_target(&mut positions, pole);
    }

    if matches!(
        settings.constraint_enforcement,
        IkConstraintEnforcement::AfterEachIteration | IkConstraintEnforcement::AfterSolve
    ) {
        positions = reconstruct_with_constraints(chain, &positions);
    }

    let error = positions.last().copied().unwrap_or(root).distance(target);
    SolveResult {
        positions,
        rotations: Vec::new(),
        error,
        unreachable,
        status: if unreachable {
            IkSolveStatus::Unreachable
        } else if error <= settings.tolerance {
            IkSolveStatus::Solved
        } else {
            IkSolveStatus::ReachedLimit
        },
    }
}
