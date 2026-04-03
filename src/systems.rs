use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;

use crate::{
    components::{
        FootPlacement, FullBodyIkRig, FullBodyIkRigState, IkChain, IkChainState, IkJoint, IkTarget,
        IkTargetAnchor, LookAtTarget, PoleTarget,
    },
    config::{IkGlobalSettings, IkSolveStatus, IkWeight},
    constraints::IkConstraint,
    math::{compute_root_offset_hint, orientation_from_axes, safe_normalize},
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
    pub suggested_root_offset: Vec3,
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

pub(crate) fn ensure_full_body_rig_state(
    mut commands: Commands,
    rigs: Query<(Entity, Option<&FullBodyIkRigState>), With<FullBodyIkRig>>,
) {
    for (entity, state) in &rigs {
        if state.is_none() {
            commands
                .entity(entity)
                .insert(FullBodyIkRigState::default());
        }
    }
}

pub(crate) fn capture_full_body_authored_roots(
    mut rigs: Query<(&FullBodyIkRig, &mut FullBodyIkRigState)>,
    roots: Query<&Transform>,
) {
    for (rig, mut state) in &mut rigs {
        let Ok(root) = roots.get(rig.root_entity) else {
            continue;
        };
        state.authored_root_translation = root.translation;
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
        Option<&LookAtTarget>,
        Option<&FootPlacement>,
    )>,
    joint_queries: Query<(Option<&IkJoint>, Option<&IkConstraint>)>,
    transform_helper: TransformHelper,
) {
    for (
        entity,
        chain,
        mut cache,
        mut state,
        target,
        target_anchor,
        pole,
        look_at,
        foot_placement,
    ) in &mut chains
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
            state.suggested_root_offset = Vec3::ZERO;
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
            look_at,
            foot_placement,
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
        state.suggested_root_offset = prepared.suggested_root_offset;

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

pub(crate) fn apply_full_body_rigs(
    mut rigs: Query<(&FullBodyIkRig, &mut FullBodyIkRigState)>,
    chain_states: Query<&IkChainState>,
    mut transforms: Query<&mut Transform>,
) {
    for (rig, mut state) in &mut rigs {
        if !rig.enabled {
            state.combined_root_offset = Vec3::ZERO;
            state.active_chains = 0;
            state.max_chain_error = 0.0;
            continue;
        }

        let mut weighted_offset = Vec3::ZERO;
        let mut total_influence = 0.0;
        let mut active_chains = 0usize;
        let mut max_chain_error = 0.0f32;

        for chain in &rig.chains {
            if !chain.influence.is_finite() || chain.influence <= 0.0 {
                continue;
            }

            let Ok(chain_state) = chain_states.get(chain.chain_entity) else {
                continue;
            };
            if !chain_state.suggested_root_offset.is_finite() {
                continue;
            }

            weighted_offset += chain_state.suggested_root_offset * chain.influence;
            total_influence += chain.influence;
            active_chains += 1;
            max_chain_error = max_chain_error.max(chain_state.last_error);
        }

        let mut combined_offset = if total_influence > 0.0 {
            weighted_offset / total_influence
        } else {
            Vec3::ZERO
        };
        let axis = safe_normalize(rig.root_axis, Vec3::Y);
        combined_offset = axis * combined_offset.dot(axis);

        let max_root_offset = rig.max_root_offset.max(0.0);
        if combined_offset.length_squared() > max_root_offset * max_root_offset
            && max_root_offset > 0.0
        {
            combined_offset = combined_offset.normalize_or_zero() * max_root_offset;
        }
        combined_offset *= rig.root_blend.clamp(0.0, 1.0);

        state.combined_root_offset = combined_offset;
        state.active_chains = active_chains;
        state.max_chain_error = max_chain_error;

        if rig.apply_translation
            && let Ok(mut root) = transforms.get_mut(rig.root_entity)
        {
            root.translation = state.authored_root_translation + combined_offset;
        }
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
    look_at: Option<&LookAtTarget>,
    foot_placement: Option<&FootPlacement>,
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
    let effector_rotation = joints
        .last()
        .map(|joint| joint.authored_rotation)
        .unwrap_or(Quat::IDENTITY);
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

    let mut suggested_root_offset = Vec3::ZERO;

    if let Some(look_at) = look_at.filter(|look_at| look_at.enabled) {
        let Some(point) = resolve_point(look_at.point, look_at.space, transform_helper) else {
            return invalidate_chain_state(state);
        };
        let direction = safe_normalize(point - root_position, Vec3::Z);
        let reach_distance = look_at.reach_distance.unwrap_or(cache.total_length);
        resolved_target.position =
            root_position + direction * reach_distance.min(cache.total_length);
        resolved_target.orientation = Some(orientation_from_axes(
            look_at.forward_axis,
            look_at.up_axis,
            direction,
            Vec3::Y,
        ));
        resolved_target.weight = look_at.weight;
    }

    if let Some(foot) = foot_placement.filter(|foot| foot.enabled) {
        let Some(contact_point) = resolve_point(foot.contact_point, foot.space, transform_helper)
        else {
            return invalidate_chain_state(state);
        };
        let contact_normal =
            resolve_vector(foot.contact_normal, foot.space, transform_helper).unwrap_or(Vec3::Y);
        let foot_up = safe_normalize(foot.foot_up_axis, Vec3::Y);
        let foot_forward = safe_normalize(foot.foot_forward_axis, Vec3::Z);
        let effector_forward = effector_rotation * foot_forward;
        let aligned_up =
            (effector_rotation * foot_up).lerp(contact_normal, foot.normal_blend.clamp(0.0, 1.0));
        let planar_forward =
            (effector_forward - aligned_up * effector_forward.dot(aligned_up)).normalize_or_zero();
        let forward = if planar_forward.length_squared() > 0.0 {
            planar_forward
        } else {
            aligned_up.any_orthonormal_vector()
        };
        resolved_target.position =
            contact_point + safe_normalize(contact_normal, Vec3::Y) * foot.ankle_offset;
        resolved_target.orientation = Some(orientation_from_axes(
            foot_forward,
            foot_up,
            forward,
            aligned_up,
        ));

        if let Some(root_hint) = foot.root_offset_hint {
            suggested_root_offset = compute_root_offset_hint(
                root_position,
                resolved_target.position,
                cache.total_length,
                root_hint.axis,
                root_hint.max_distance,
                root_hint.weight,
            );
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
        suggested_root_offset,
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
    state.suggested_root_offset = Vec3::ZERO;
    state.total_length = 0.0;
    None
}

fn resolve_point(
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

fn resolve_vector(
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

fn resolve_rotation(
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
