mod ccd;
mod fabrik;
mod two_bone;

use bevy::prelude::*;

use crate::{
    components::IkJoint,
    config::{IkSolveSettings, IkSolveStatus, IkSolver},
    constraints::IkConstraint,
    math::{align_axis_rotation, safe_normalize},
};

pub use ccd::solve_ccd;
pub use fabrik::solve_fabrik;
pub use two_bone::solve_two_bone;

#[derive(Clone, Debug)]
pub struct SolverJointState {
    pub position: Vec3,
    pub authored_rotation: Quat,
    pub settings: IkJoint,
    pub constraint: Option<IkConstraint>,
}

#[derive(Clone, Debug)]
pub struct SolverChainState {
    pub joints: Vec<SolverJointState>,
    pub lengths: Vec<f32>,
}

impl SolverChainState {
    pub fn total_length(&self) -> f32 {
        self.lengths.iter().sum()
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.joints.len() < 2 {
            return Err("IK chains need at least two joints");
        }
        if self.lengths.len() != self.joints.len().saturating_sub(1) {
            return Err("Segment length count must be joints - 1");
        }
        if self
            .lengths
            .iter()
            .any(|length| !length.is_finite() || *length <= 0.0)
        {
            return Err("All segment lengths must be finite and positive");
        }
        Ok(())
    }

    pub fn positions(&self) -> Vec<Vec3> {
        self.joints.iter().map(|joint| joint.position).collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedTarget {
    pub position: Vec3,
    pub orientation: Option<Quat>,
    pub position_weight: f32,
    pub rotation_weight: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedPole {
    pub point: Vec3,
    pub weight: f32,
}

#[derive(Clone, Debug)]
pub struct SolveResult {
    pub positions: Vec<Vec3>,
    pub rotations: Vec<Quat>,
    pub error: f32,
    pub unreachable: bool,
    pub status: IkSolveStatus,
}

impl SolveResult {
    pub fn invalid(chain: &SolverChainState) -> Self {
        Self {
            positions: chain.positions(),
            rotations: chain
                .joints
                .iter()
                .map(|joint| joint.authored_rotation)
                .collect(),
            error: 0.0,
            unreachable: false,
            status: IkSolveStatus::InvalidChain,
        }
    }
}

pub fn solve_chain(
    solver: IkSolver,
    chain: &SolverChainState,
    target: ResolvedTarget,
    pole: Option<ResolvedPole>,
    settings: IkSolveSettings,
) -> SolveResult {
    if chain.validate().is_err() {
        return SolveResult::invalid(chain);
    }

    let current_positions = chain.positions();
    let current_effector = *current_positions.last().unwrap_or(&Vec3::ZERO);
    let desired_target =
        current_effector.lerp(target.position, target.position_weight.clamp(0.0, 1.0));

    let mut result = match solver {
        IkSolver::Fabrik => solve_fabrik(chain, desired_target, pole, settings),
        IkSolver::Ccd => solve_ccd(chain, desired_target, pole, settings),
        IkSolver::TwoBone => solve_two_bone(chain, desired_target, pole, settings),
    };

    apply_damping(chain, &mut result.positions);
    result.positions = reconstruct_with_constraints(chain, &result.positions);
    result.rotations = build_rotations(
        chain,
        &result.positions,
        target.orientation,
        target.rotation_weight,
    );
    result.error = result
        .positions
        .last()
        .copied()
        .unwrap_or(Vec3::ZERO)
        .distance(target.position);

    if matches!(result.status, IkSolveStatus::ReachedLimit) && result.error <= settings.tolerance {
        result.status = IkSolveStatus::Solved;
    }

    result
}

pub(crate) fn apply_pole_target(positions: &mut [Vec3], pole: ResolvedPole) {
    if positions.len() < 3 || pole.weight <= 0.0 {
        return;
    }

    let root = positions[0];
    let effector = *positions.last().unwrap_or(&root);
    let chain_axis = safe_normalize(effector - root, Vec3::Y);
    let desired = safe_normalize(
        crate::math::project_on_plane(pole.point - root, chain_axis),
        chain_axis.any_orthonormal_vector(),
    );

    for index in 1..positions.len() - 1 {
        let current = safe_normalize(
            crate::math::project_on_plane(positions[index] - root, chain_axis),
            desired,
        );
        let angle = crate::math::signed_angle_on_axis(current, desired, chain_axis)
            * pole.weight.clamp(0.0, 1.0);
        let rotation = Quat::from_axis_angle(chain_axis, angle);
        positions[index] = root + rotation * (positions[index] - root);
    }
}

pub(crate) fn reconstruct_with_constraints(
    chain: &SolverChainState,
    positions: &[Vec3],
) -> Vec<Vec3> {
    let mut output = vec![Vec3::ZERO; positions.len()];
    output[0] = positions[0];

    for index in 0..chain.lengths.len() {
        let joint = &chain.joints[index];
        let authored_axis =
            joint.authored_rotation * safe_normalize(joint.settings.tip_axis, Vec3::Y);
        let desired = safe_normalize(positions[index + 1] - output[index], authored_axis);
        let constrained = joint
            .constraint
            .as_ref()
            .map(|constraint| {
                constraint.constrain(desired, joint.authored_rotation, joint.settings.tip_axis)
            })
            .unwrap_or(desired);
        let stiffness = joint.settings.stiffness.clamp(0.0, 1.0);
        let blended = safe_normalize(
            authored_axis.lerp(constrained, 1.0 - stiffness),
            constrained,
        );
        output[index + 1] = output[index] + blended * chain.lengths[index];
    }

    output
}

fn build_rotations(
    chain: &SolverChainState,
    positions: &[Vec3],
    orientation_target: Option<Quat>,
    rotation_weight: f32,
) -> Vec<Quat> {
    let mut rotations = chain
        .joints
        .iter()
        .map(|joint| joint.authored_rotation)
        .collect::<Vec<_>>();

    for index in 0..chain.lengths.len() {
        let desired = safe_normalize(
            positions[index + 1] - positions[index],
            chain.joints[index].authored_rotation * chain.joints[index].settings.tip_axis,
        );
        rotations[index] = align_axis_rotation(
            chain.joints[index].authored_rotation,
            chain.joints[index].settings.tip_axis,
            desired,
        );
    }

    if let (Some(orientation_target), Some(last_rotation)) =
        (orientation_target, rotations.last_mut())
    {
        *last_rotation = last_rotation.slerp(orientation_target, rotation_weight.clamp(0.0, 1.0));
    }

    rotations
}

fn apply_damping(chain: &SolverChainState, positions: &mut [Vec3]) {
    for (index, joint) in chain.joints.iter().enumerate().skip(1) {
        let damping = joint.settings.damping.clamp(0.0, 1.0);
        positions[index] = joint.position.lerp(positions[index], damping);
    }
}
