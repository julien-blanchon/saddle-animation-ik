use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;

use crate::{
    components::{IkChain, IkChainState, IkJoint, IkTarget, IkTargetAnchor, PoleTarget},
    config::{IkGlobalSettings, IkSolveStatus, IkWeight},
    constraints::IkConstraint,
    solver::{ResolvedPole, ResolvedTarget, SolverChainState, SolverJointState, solve_chain},
};

#[derive(Resource, Default)]
pub(crate) struct IkRuntimeState {
    pub active: bool,
}

#[derive(Component, Clone, Debug)]
pub(crate) struct IkChainCache {
    pub lengths: Vec<f32>,
    pub total_length: f32,
    pub invalid_logged: bool,
}

#[derive(Component, Clone, Debug)]
pub(crate) struct IkPreparedChain {
    pub joints: Vec<SolverJointState>,
    pub positions: Vec<Vec3>,
    pub lengths: Vec<f32>,
    pub constraints: Vec<Option<IkConstraint>>,
    pub target: ResolvedTarget,
    pub pole: Option<ResolvedPole>,
    pub total_length: f32,
}

#[derive(Component, Clone, Debug)]
pub(crate) struct IkSolvedChain {
    pub positions: Vec<Vec3>,
    pub rotations: Vec<Quat>,
    pub error: f32,
    pub unreachable: bool,
    pub status: IkSolveStatus,
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) enum IkTargetOverride {
    Valid {
        position: Vec3,
        orientation: Option<Quat>,
        weight_override: Option<IkWeight>,
    },
    Invalid,
}

pub(crate) fn activate_runtime(mut runtime: ResMut<IkRuntimeState>) {
    runtime.active = true;
}

pub(crate) fn deactivate_runtime(mut runtime: ResMut<IkRuntimeState>) {
    runtime.active = false;
}

pub(crate) fn runtime_is_active(runtime: Res<IkRuntimeState>) -> bool {
    runtime.active
}

pub(crate) fn ensure_chain_state(
    mut commands: Commands,
    chains: Query<(Entity, Option<&IkChainState>, Option<&IkChainCache>), With<IkChain>>,
) {
    for (entity, state, cache) in &chains {
        if state.is_none() {
            commands.entity(entity).insert(IkChainState::default());
        }
        if cache.is_none() {
            commands.entity(entity).insert(IkChainCache {
                lengths: Vec::new(),
                total_length: 0.0,
                invalid_logged: false,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_chains(
    mut commands: Commands,
    globals: Res<IkGlobalSettings>,
    mut chains: Query<(
        Entity,
        &IkChain,
        &mut IkChainCache,
        &mut IkChainState,
        Option<&IkTarget>,
        Option<&IkTargetAnchor>,
        Option<&PoleTarget>,
        Option<&IkTargetOverride>,
    )>,
    joint_queries: Query<(Option<&IkJoint>, Option<&IkConstraint>)>,
    transform_helper: TransformHelper,
) {
    for (entity, chain, mut cache, mut state, target, target_anchor, pole, target_override) in
        &mut chains
    {
        state.status = if chain.enabled {
            IkSolveStatus::ReachedLimit
        } else {
            IkSolveStatus::Disabled
        };

        if !chain.enabled {
            state.cache_ready = false;
            state.last_error = 0.0;
            state.unreachable = false;
            commands
                .entity(entity)
                .remove::<IkPreparedChain>()
                .remove::<IkSolvedChain>();
            continue;
        }

        let Some(prepared) = prepare_single_chain(
            chain,
            &mut cache,
            &mut state,
            target,
            target_anchor,
            pole,
            target_override,
            &joint_queries,
            &transform_helper,
            &globals,
        ) else {
            commands
                .entity(entity)
                .remove::<IkPreparedChain>()
                .remove::<IkSolvedChain>();
            continue;
        };

        state.cache_ready = true;
        state.total_length = prepared.total_length;
        state.target_position = prepared.target.position;

        commands.entity(entity).insert(prepared);
    }
}

pub(crate) fn solve_chains(
    mut commands: Commands,
    globals: Res<IkGlobalSettings>,
    chains: Query<(Entity, &IkChain, &IkPreparedChain)>,
) {
    for (entity, chain, prepared) in &chains {
        let solve = chain.solve.sanitized(globals.default_solve);
        let solver_chain = SolverChainState {
            joints: prepared.joints.clone(),
            lengths: prepared.lengths.clone(),
        };
        let result = solve_chain(
            chain.solver,
            &solver_chain,
            prepared.target,
            prepared.pole,
            solve,
        );
        commands.entity(entity).insert(IkSolvedChain {
            positions: result.positions,
            rotations: result.rotations,
            error: result.error,
            unreachable: result.unreachable,
            status: result.status,
        });
    }
}

pub(crate) fn apply_chains(
    mut chains: Query<(Entity, &IkChain, &mut IkChainState, Option<&IkSolvedChain>)>,
    parents: Query<&ChildOf>,
    mut transforms: ParamSet<(TransformHelper, Query<&mut Transform>)>,
) {
    for (entity, chain, mut state, solved) in &mut chains {
        let Some(solved) = solved else {
            continue;
        };

        state.status = solved.status;
        state.last_error = solved.error;
        state.unreachable = solved.unreachable;
        state.effector_position = solved.positions.last().copied().unwrap_or(Vec3::ZERO);

        if matches!(
            solved.status,
            IkSolveStatus::InvalidChain | IkSolveStatus::Disabled
        ) {
            continue;
        }

        let desired_world = chain
            .joints
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, joint_entity)| {
                let target_translation = solved.positions.get(index).copied()?;
                let target_rotation = solved
                    .rotations
                    .get(index)
                    .copied()
                    .unwrap_or(Quat::IDENTITY);

                Some((
                    joint_entity,
                    Affine3A::from_scale_rotation_translation(
                        Vec3::ONE,
                        target_rotation,
                        target_translation,
                    ),
                ))
            })
            .collect::<HashMap<_, _>>();

        for (index, joint_entity) in chain.joints.iter().copied().enumerate() {
            let Some(target_translation) = solved.positions.get(index).copied() else {
                continue;
            };
            let target_rotation = solved
                .rotations
                .get(index)
                .copied()
                .unwrap_or(Quat::IDENTITY);

            let local_scale = {
                let transform_query = transforms.p1();
                transform_query
                    .get(joint_entity)
                    .map(|transform| transform.scale)
                    .unwrap_or(Vec3::ONE)
            };

            let desired_joint_world = Affine3A::from_scale_rotation_translation(
                local_scale,
                target_rotation,
                target_translation,
            );

            let local_affine = if let Ok(child_of) = parents.get(joint_entity) {
                if let Some(parent_world) = desired_world.get(&child_of.parent()) {
                    parent_world.inverse() * desired_joint_world
                } else if let Ok(parent_world) = transforms
                    .p0()
                    .compute_global_transform(child_of.parent())
                    .map(|global| global.affine())
                {
                    parent_world.inverse() * desired_joint_world
                } else {
                    desired_joint_world
                }
            } else {
                desired_joint_world
            };

            let mut transform_query = transforms.p1();
            let Ok(mut local) = transform_query.get_mut(joint_entity) else {
                continue;
            };

            *local = Transform::from_matrix(Mat4::from(local_affine));
        }

        // Keep the state component in the world even if the prepared chain vanished.
        let _ = entity;
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare_single_chain(
    chain: &IkChain,
    cache: &mut IkChainCache,
    state: &mut IkChainState,
    target: Option<&IkTarget>,
    target_anchor: Option<&IkTargetAnchor>,
    pole: Option<&PoleTarget>,
    target_override: Option<&IkTargetOverride>,
    joint_queries: &Query<(Option<&IkJoint>, Option<&IkConstraint>)>,
    transform_helper: &TransformHelper,
    globals: &IkGlobalSettings,
) -> Option<IkPreparedChain> {
    let mut joints = Vec::with_capacity(chain.joints.len());
    let mut positions = Vec::with_capacity(chain.joints.len());
    let mut constraints = Vec::with_capacity(chain.joints.len());

    for joint_entity in &chain.joints {
        let Ok((joint, constraint)) = joint_queries.get(*joint_entity) else {
            return invalidate_chain_state(state);
        };
        let Ok(global) = transform_helper.compute_global_transform(*joint_entity) else {
            return invalidate_chain_state(state);
        };

        let transform = global.compute_transform();
        let settings = joint.copied().unwrap_or_default();
        positions.push(transform.translation);
        constraints.push(constraint.cloned());
        joints.push(SolverJointState {
            position: transform.translation,
            authored_rotation: transform.rotation,
            settings,
            constraint: constraint.cloned(),
        });
    }

    if positions.len() < 2 {
        return invalidate_chain_state(state);
    }

    if cache.lengths.len() != positions.len().saturating_sub(1) {
        cache.lengths.clear();
        for index in 0..positions.len() - 1 {
            cache
                .lengths
                .push(positions[index].distance(positions[index + 1]));
        }
        cache.total_length = cache.lengths.iter().sum();
        cache.invalid_logged = false;
    } else if !globals.preserve_initial_lengths {
        for index in 0..positions.len() - 1 {
            cache.lengths[index] = positions[index].distance(positions[index + 1]);
        }
        cache.total_length = cache.lengths.iter().sum();
    }

    if cache
        .lengths
        .iter()
        .any(|length| !length.is_finite() || *length <= 0.0)
    {
        if globals.log_invalid_chains_once && !cache.invalid_logged {
            warn!(
                "ik: invalid zero-length segment in chain with {} joints",
                positions.len()
            );
            cache.invalid_logged = true;
        }
        return invalidate_chain_state(state);
    }

    let root_position = positions[0];
    let effector_position = *positions.last().unwrap_or(&root_position);
    let target_active = target.is_none_or(|target| target.enabled);
    let mut resolved_target = if let Some(target) = target.filter(|target| target.enabled) {
        let Some(position) = resolve_point(target.position, target.space, transform_helper) else {
            return invalidate_chain_state(state);
        };
        let orientation = match target.orientation {
            Some(orientation) => {
                match resolve_rotation(orientation, target.space, transform_helper) {
                    Some(orientation) => Some(orientation),
                    None => return invalidate_chain_state(state),
                }
            }
            None => None,
        };

        IkTarget {
            enabled: true,
            position,
            orientation,
            space: crate::config::IkTargetSpace::World,
            weight: target.weight,
        }
    } else {
        IkTarget {
            enabled: target_active,
            position: effector_position,
            orientation: None,
            space: crate::config::IkTargetSpace::World,
            weight: if target_active {
                target.map(|target| target.weight).unwrap_or_default()
            } else {
                IkWeight {
                    overall: 0.0,
                    position: 0.0,
                    rotation: 0.0,
                }
            },
        }
    };

    if target_active {
        if let Some(anchor) = target_anchor {
            if let Ok(global) = transform_helper.compute_global_transform(anchor.entity) {
                let base = global.compute_transform();
                resolved_target.position = base.transform_point(anchor.translation_offset);
                let anchor_rotation = base.rotation * anchor.rotation_offset;
                if resolved_target.orientation.is_none() {
                    resolved_target.orientation = Some(anchor_rotation);
                }
            }
        }
    }

    if let Some(target_override) = target_override {
        match *target_override {
            IkTargetOverride::Valid {
                position,
                orientation,
                weight_override,
            } => {
                resolved_target.position = position;
                resolved_target.orientation = orientation;
                if let Some(weight_override) = weight_override {
                    resolved_target.weight = weight_override;
                }
            }
            IkTargetOverride::Invalid => return invalidate_chain_state(state),
        }
    }

    let position_weight =
        (chain.weight.position_factor() * resolved_target.weight.position_factor()).clamp(0.0, 1.0);
    let rotation_weight =
        (chain.weight.rotation_factor() * resolved_target.weight.rotation_factor()).clamp(0.0, 1.0);

    let pole = if let Some(pole) = pole.filter(|pole| pole.enabled) {
        let Some(point) = resolve_point(pole.point, pole.space, transform_helper) else {
            return invalidate_chain_state(state);
        };

        Some(ResolvedPole {
            point,
            weight: pole.weight.clamp(0.0, 1.0),
        })
    } else {
        None
    };

    if !resolved_target.position.is_finite() {
        return invalidate_chain_state(state);
    }

    Some(IkPreparedChain {
        joints,
        positions,
        lengths: cache.lengths.clone(),
        constraints,
        target: ResolvedTarget {
            position: resolved_target.position,
            orientation: resolved_target.orientation,
            position_weight,
            rotation_weight,
        },
        pole,
        total_length: cache.total_length,
    })
}

fn invalidate_chain_state(state: &mut IkChainState) -> Option<IkPreparedChain> {
    state.status = IkSolveStatus::InvalidChain;
    state.cache_ready = false;
    state.last_error = 0.0;
    state.unreachable = false;
    state.target_position = Vec3::ZERO;
    state.effector_position = Vec3::ZERO;
    state.total_length = 0.0;
    None
}

pub(crate) fn resolve_point(
    point: Vec3,
    space: crate::config::IkTargetSpace,
    transform_helper: &TransformHelper,
) -> Option<Vec3> {
    if !point.is_finite() {
        return None;
    }

    match space {
        crate::config::IkTargetSpace::World => Some(point),
        crate::config::IkTargetSpace::LocalToEntity(entity) => transform_helper
            .compute_global_transform(entity)
            .ok()
            .map(|global| global.compute_transform().transform_point(point)),
    }
}

pub(crate) fn resolve_vector(
    vector: Vec3,
    space: crate::config::IkTargetSpace,
    transform_helper: &TransformHelper,
) -> Option<Vec3> {
    if !vector.is_finite() {
        return None;
    }

    match space {
        crate::config::IkTargetSpace::World => Some(vector),
        crate::config::IkTargetSpace::LocalToEntity(entity) => transform_helper
            .compute_global_transform(entity)
            .ok()
            .map(|global| global.compute_transform().rotation * vector),
    }
}

pub(crate) fn resolve_rotation(
    rotation: Quat,
    space: crate::config::IkTargetSpace,
    transform_helper: &TransformHelper,
) -> Option<Quat> {
    if !rotation.is_finite() {
        return None;
    }

    let rotation = rotation.normalize();
    match space {
        crate::config::IkTargetSpace::World => Some(rotation),
        crate::config::IkTargetSpace::LocalToEntity(entity) => transform_helper
            .compute_global_transform(entity)
            .ok()
            .map(|global| (global.compute_transform().rotation * rotation).normalize()),
    }
}
