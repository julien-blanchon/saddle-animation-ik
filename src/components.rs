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
            total_length: 0.0,
        }
    }
}
