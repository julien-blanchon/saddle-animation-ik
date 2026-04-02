use bevy::prelude::*;

use crate::{
    config::{IkConstraintEnforcement, IkSolveSettings, IkSolveStatus},
    math::safe_normalize,
};

use super::{
    ResolvedPole, SolveResult, SolverChainState, apply_pole_target, reconstruct_with_constraints,
};

pub fn solve_fabrik(
    chain: &SolverChainState,
    target: Vec3,
    pole: Option<ResolvedPole>,
    settings: IkSolveSettings,
) -> SolveResult {
    let mut positions = chain.positions();
    let root = positions[0];
    let total_length = chain.total_length();
    let distance_to_target = root.distance(target);
    let unreachable = distance_to_target >= total_length;

    if unreachable {
        let direction = safe_normalize(target - root, Vec3::Y);
        for index in 0..chain.lengths.len() {
            positions[index + 1] = positions[index] + direction * chain.lengths[index];
        }
        if let Some(pole) = pole {
            apply_pole_target(&mut positions, pole);
        }
        positions = reconstruct_with_constraints(chain, &positions);
        let error = positions.last().copied().unwrap_or(root).distance(target);
        return SolveResult {
            positions,
            rotations: Vec::new(),
            error,
            unreachable: true,
            status: IkSolveStatus::Unreachable,
        };
    }

    let mut status = IkSolveStatus::ReachedLimit;
    for _ in 0..settings.iterations {
        let last = positions.len() - 1;
        positions[last] = target;

        for index in (0..last).rev() {
            let direction = safe_normalize(positions[index] - positions[index + 1], Vec3::Y);
            positions[index] = positions[index + 1] + direction * chain.lengths[index];
        }

        positions[0] = root;
        for index in 0..last {
            let direction = safe_normalize(positions[index + 1] - positions[index], Vec3::Y);
            positions[index + 1] = positions[index] + direction * chain.lengths[index];
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

        let error = positions[last].distance(target);
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

    let error = positions.last().copied().unwrap_or(root).distance(target);
    SolveResult {
        positions,
        rotations: Vec::new(),
        error,
        unreachable: false,
        status,
    }
}
