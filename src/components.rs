use bevy::prelude::*;

use crate::config::{IkSolveSettings, IkSolveStatus, IkSolver, IkTargetSpace, IkWeight};

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct IkChain {
    pub joints: Vec<Entity>,
    pub enabled: bool,
    pub solver: IkSolver,
    pub solve: IkSolveSettings,
    pub weight: IkWeight,
}

impl Default for IkChain {
    fn default() -> Self {
        Self {
            joints: Vec::new(),
            enabled: true,
            solver: IkSolver::Fabrik,
            solve: IkSolveSettings::default(),
            weight: IkWeight::default(),
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct IkJoint {
    pub tip_axis: Vec3,
    pub pole_axis: Vec3,
    pub stiffness: f32,
    pub damping: f32,
}

impl Default for IkJoint {
    fn default() -> Self {
        Self {
            tip_axis: Vec3::Y,
            pole_axis: Vec3::Z,
            stiffness: 0.0,
            damping: 1.0,
        }
    }
}

#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct IkTarget {
    pub enabled: bool,
    pub position: Vec3,
    pub orientation: Option<Quat>,
    pub space: IkTargetSpace,
    pub weight: IkWeight,
}

impl Default for IkTarget {
    fn default() -> Self {
        Self {
            enabled: true,
            position: Vec3::ZERO,
            orientation: None,
            space: IkTargetSpace::World,
            weight: IkWeight::default(),
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct IkTargetAnchor {
    pub entity: Entity,
    pub translation_offset: Vec3,
    pub rotation_offset: Quat,
}

#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component)]
pub struct PoleTarget {
    pub enabled: bool,
    pub point: Vec3,
    pub space: IkTargetSpace,
    pub weight: f32,
}

impl Default for PoleTarget {
    fn default() -> Self {
        Self {
            enabled: true,
            point: Vec3::ZERO,
            space: IkTargetSpace::World,
            weight: 1.0,
        }
    }
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

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct IkDebugDraw {
    pub enabled: bool,
    pub color: Color,
    pub joint_radius: f32,
    pub draw_constraints: bool,
}

impl Default for IkDebugDraw {
    fn default() -> Self {
        Self {
            enabled: true,
            color: Color::srgb(0.22, 0.88, 0.95),
            joint_radius: 0.05,
            draw_constraints: true,
        }
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct IkChainState {
    pub status: IkSolveStatus,
    pub cache_ready: bool,
    pub last_error: f32,
    pub unreachable: bool,
    pub target_position: Vec3,
    pub effector_position: Vec3,
    pub suggested_root_offset: Vec3,
    pub total_length: f32,
}

impl Default for IkChainState {
    fn default() -> Self {
        Self {
            status: IkSolveStatus::Disabled,
            cache_ready: false,
            last_error: 0.0,
            unreachable: false,
            target_position: Vec3::ZERO,
            effector_position: Vec3::ZERO,
            suggested_root_offset: Vec3::ZERO,
            total_length: 0.0,
        }
    }
}
