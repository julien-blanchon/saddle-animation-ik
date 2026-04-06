use bevy::{
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
    transform::helper::TransformHelper,
};

use crate::{
    IkSystems,
    components::{IkChain, IkChainState},
    config::{IkTargetSpace, IkWeight},
    math::{compute_root_offset_hint, orientation_from_axes, safe_normalize},
    systems::{self, IkPreparedChain, IkTargetOverride},
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum IkRigHelperSystems {
    ResolveTargets,
    UpdateHints,
    ApplyRigs,
}

#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct RootOffsetHint {
    pub axis: Vec3,
    pub max_distance: f32,
    pub weight: f32,
}

impl Default for RootOffsetHint {
    fn default() -> Self {
        Self {
            axis: Vec3::Y,
            max_distance: 0.35,
            weight: 1.0,
        }
    }
}

#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct FootPlacement {
    pub enabled: bool,
    pub contact_point: Vec3,
    pub contact_normal: Vec3,
    pub space: IkTargetSpace,
    pub ankle_offset: f32,
    pub foot_up_axis: Vec3,
    pub foot_forward_axis: Vec3,
    pub normal_blend: f32,
    pub root_offset_hint: Option<RootOffsetHint>,
}

impl Default for FootPlacement {
    fn default() -> Self {
        Self {
            enabled: true,
            contact_point: Vec3::ZERO,
            contact_normal: Vec3::Y,
            space: IkTargetSpace::World,
            ankle_offset: 0.02,
            foot_up_axis: Vec3::Y,
            foot_forward_axis: Vec3::Z,
            normal_blend: 1.0,
            root_offset_hint: None,
        }
    }
}

#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct LookAtTarget {
    pub enabled: bool,
    pub point: Vec3,
    pub space: IkTargetSpace,
    pub forward_axis: Vec3,
    pub up_axis: Vec3,
    pub reach_distance: Option<f32>,
    pub weight: IkWeight,
}

impl Default for LookAtTarget {
    fn default() -> Self {
        Self {
            enabled: true,
            point: Vec3::ZERO,
            space: IkTargetSpace::World,
            forward_axis: Vec3::Z,
            up_axis: Vec3::Y,
            reach_distance: None,
            weight: IkWeight::default(),
        }
    }
}

#[derive(Component, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct IkRootOffsetState {
    pub suggested_root_offset: Vec3,
}

#[derive(Clone, Debug, Reflect, PartialEq)]
pub struct FullBodyIkChain {
    pub chain_entity: Entity,
    pub influence: f32,
}

impl FullBodyIkChain {
    pub fn new(chain_entity: Entity) -> Self {
        Self {
            chain_entity,
            influence: 1.0,
        }
    }

    pub fn with_influence(mut self, influence: f32) -> Self {
        self.influence = influence;
        self
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct FullBodyIkRig {
    pub enabled: bool,
    pub root_entity: Entity,
    pub chains: Vec<FullBodyIkChain>,
    pub root_axis: Vec3,
    pub max_root_offset: f32,
    pub root_blend: f32,
    pub apply_translation: bool,
}

impl FullBodyIkRig {
    pub fn new(root_entity: Entity) -> Self {
        Self {
            enabled: true,
            root_entity,
            chains: Vec::new(),
            root_axis: Vec3::Y,
            max_root_offset: 0.45,
            root_blend: 1.0,
            apply_translation: true,
        }
    }

    pub fn with_chain(mut self, chain_entity: Entity) -> Self {
        self.chains.push(FullBodyIkChain::new(chain_entity));
        self
    }

    pub fn with_root_axis(mut self, root_axis: Vec3) -> Self {
        self.root_axis = root_axis;
        self
    }

    pub fn with_max_root_offset(mut self, max_root_offset: f32) -> Self {
        self.max_root_offset = max_root_offset;
        self
    }

    pub fn with_root_blend(mut self, root_blend: f32) -> Self {
        self.root_blend = root_blend;
        self
    }

    pub fn without_translation_apply(mut self) -> Self {
        self.apply_translation = false;
        self
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct FullBodyIkRigState {
    pub authored_root_translation: Vec3,
    pub combined_root_offset: Vec3,
    pub active_chains: usize,
    pub max_chain_error: f32,
}

impl Default for FullBodyIkRigState {
    fn default() -> Self {
        Self {
            authored_root_translation: Vec3::ZERO,
            combined_root_offset: Vec3::ZERO,
            active_chains: 0,
            max_chain_error: 0.0,
        }
    }
}

pub struct IkRigHelpersPlugin {
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl IkRigHelpersPlugin {
    pub fn new(update_schedule: impl ScheduleLabel) -> Self {
        Self {
            update_schedule: update_schedule.intern(),
        }
    }
}

impl Default for IkRigHelpersPlugin {
    fn default() -> Self {
        Self::new(Update)
    }
}

impl Plugin for IkRigHelpersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<systems::IkRuntimeState>()
            .register_type::<FootPlacement>()
            .register_type::<FullBodyIkChain>()
            .register_type::<FullBodyIkRig>()
            .register_type::<FullBodyIkRigState>()
            .register_type::<IkRootOffsetState>()
            .register_type::<LookAtTarget>()
            .register_type::<RootOffsetHint>();

        app.configure_sets(
            self.update_schedule,
            IkRigHelperSystems::ResolveTargets.before(IkSystems::Prepare),
        )
        .configure_sets(
            self.update_schedule,
            IkRigHelperSystems::UpdateHints
                .after(IkSystems::Prepare)
                .before(IkSystems::Solve),
        )
        .configure_sets(
            self.update_schedule,
            IkRigHelperSystems::ApplyRigs.after(IkSystems::Apply),
        )
        .add_systems(
            self.update_schedule,
            (
                (
                    ensure_full_body_rig_state,
                    capture_full_body_authored_roots,
                    resolve_helper_targets,
                )
                    .chain()
                    .in_set(IkRigHelperSystems::ResolveTargets),
                update_root_offset_states.in_set(IkRigHelperSystems::UpdateHints),
                apply_full_body_rigs.in_set(IkRigHelperSystems::ApplyRigs),
            )
                .run_if(systems::runtime_is_active),
        );
    }
}

fn ensure_full_body_rig_state(
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

fn capture_full_body_authored_roots(
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

fn resolve_helper_targets(
    mut commands: Commands,
    chains: Query<(
        Entity,
        &IkChain,
        Option<&LookAtTarget>,
        Option<&FootPlacement>,
    )>,
    transform_helper: TransformHelper,
) {
    for (entity, chain, look_at, foot) in &chains {
        let look_at = look_at.filter(|look_at| look_at.enabled);
        let foot = foot.filter(|foot| foot.enabled);

        if look_at.is_none() && foot.is_none() {
            commands.entity(entity).remove::<IkTargetOverride>();
            continue;
        }

        let Some((root_position, effector_rotation, total_length)) =
            chain_context(chain, &transform_helper)
        else {
            commands.entity(entity).insert(IkTargetOverride::Invalid);
            continue;
        };

        let mut target_override: Option<(Vec3, Option<Quat>, Option<IkWeight>)> = None;

        if let Some(look_at) = look_at {
            let Some(point) =
                systems::resolve_point(look_at.point, look_at.space, &transform_helper)
            else {
                commands.entity(entity).insert(IkTargetOverride::Invalid);
                continue;
            };

            let direction = safe_normalize(point - root_position, Vec3::Z);
            let reach_distance = look_at.reach_distance.unwrap_or(total_length).max(0.0);
            target_override = Some((
                root_position + direction * reach_distance.min(total_length),
                Some(orientation_from_axes(
                    look_at.forward_axis,
                    look_at.up_axis,
                    direction,
                    Vec3::Y,
                )),
                Some(look_at.weight),
            ));
        }

        if let Some(foot) = foot {
            let Some(contact_point) =
                systems::resolve_point(foot.contact_point, foot.space, &transform_helper)
            else {
                commands.entity(entity).insert(IkTargetOverride::Invalid);
                continue;
            };

            let contact_normal =
                systems::resolve_vector(foot.contact_normal, foot.space, &transform_helper)
                    .unwrap_or(Vec3::Y);
            let foot_up = safe_normalize(foot.foot_up_axis, Vec3::Y);
            let foot_forward = safe_normalize(foot.foot_forward_axis, Vec3::Z);
            let effector_forward = effector_rotation * foot_forward;
            let aligned_up = (effector_rotation * foot_up)
                .lerp(contact_normal, foot.normal_blend.clamp(0.0, 1.0));
            let planar_forward = (effector_forward - aligned_up * effector_forward.dot(aligned_up))
                .normalize_or_zero();
            let forward = if planar_forward.length_squared() > 0.0 {
                planar_forward
            } else {
                aligned_up.any_orthonormal_vector()
            };
            let weight_override = target_override
                .as_ref()
                .and_then(|(_, _, weight_override)| *weight_override);
            target_override = Some((
                contact_point + safe_normalize(contact_normal, Vec3::Y) * foot.ankle_offset,
                Some(orientation_from_axes(
                    foot_forward,
                    foot_up,
                    forward,
                    aligned_up,
                )),
                weight_override,
            ));
        }

        if let Some((position, orientation, weight_override)) = target_override {
            commands.entity(entity).insert(IkTargetOverride::Valid {
                position,
                orientation,
                weight_override,
            });
        } else {
            commands.entity(entity).remove::<IkTargetOverride>();
        }
    }
}

fn update_root_offset_states(
    mut commands: Commands,
    chains: Query<(Entity, Option<&FootPlacement>, Option<&IkPreparedChain>), With<IkChain>>,
) {
    for (entity, foot, prepared) in &chains {
        let Some(foot) = foot.filter(|foot| foot.enabled) else {
            commands.entity(entity).remove::<IkRootOffsetState>();
            continue;
        };
        let Some(root_hint) = foot.root_offset_hint else {
            commands.entity(entity).remove::<IkRootOffsetState>();
            continue;
        };
        let Some(prepared) = prepared else {
            commands.entity(entity).remove::<IkRootOffsetState>();
            continue;
        };

        commands.entity(entity).insert(IkRootOffsetState {
            suggested_root_offset: compute_root_offset_hint(
                prepared.positions[0],
                prepared.target.position,
                prepared.total_length,
                root_hint.axis,
                root_hint.max_distance,
                root_hint.weight,
            ),
        });
    }
}

fn apply_full_body_rigs(
    mut rigs: Query<(&FullBodyIkRig, &mut FullBodyIkRigState)>,
    chain_states: Query<&IkChainState>,
    root_offsets: Query<&IkRootOffsetState>,
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

            let Ok(root_offset) = root_offsets.get(chain.chain_entity) else {
                continue;
            };
            if !root_offset.suggested_root_offset.is_finite() {
                continue;
            }

            weighted_offset += root_offset.suggested_root_offset * chain.influence;
            total_influence += chain.influence;
            active_chains += 1;

            if let Ok(chain_state) = chain_states.get(chain.chain_entity) {
                max_chain_error = max_chain_error.max(chain_state.last_error);
            }
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

fn chain_context(chain: &IkChain, transform_helper: &TransformHelper) -> Option<(Vec3, Quat, f32)> {
    let mut positions = Vec::with_capacity(chain.joints.len());
    let mut effector_rotation = Quat::IDENTITY;

    for (index, joint_entity) in chain.joints.iter().copied().enumerate() {
        let Ok(global) = transform_helper.compute_global_transform(joint_entity) else {
            return None;
        };

        let transform = global.compute_transform();
        positions.push(transform.translation);
        if index + 1 == chain.joints.len() {
            effector_rotation = transform.rotation;
        }
    }

    if positions.len() < 2 {
        return None;
    }

    let mut total_length = 0.0;
    for window in positions.windows(2) {
        let length = window[0].distance(window[1]);
        if !length.is_finite() || length <= 0.0 {
            return None;
        }
        total_length += length;
    }

    Some((positions[0], effector_rotation, total_length))
}
