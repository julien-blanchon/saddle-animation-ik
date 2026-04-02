use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq, Hash)]
pub enum IkSolver {
    Fabrik,
    Ccd,
    TwoBone,
}

#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq, Hash)]
pub enum IkConstraintEnforcement {
    AfterEachIteration,
    AfterSolve,
}

#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq, Hash)]
pub enum IkSolveStatus {
    Disabled,
    Solved,
    ReachedLimit,
    Unreachable,
    InvalidChain,
}

#[derive(Clone, Copy, Debug, Reflect, PartialEq)]
pub struct IkSolveSettings {
    pub iterations: usize,
    pub tolerance: f32,
    pub constraint_iterations: usize,
    pub constraint_enforcement: IkConstraintEnforcement,
}

impl Default for IkSolveSettings {
    fn default() -> Self {
        Self {
            iterations: 12,
            tolerance: 0.01,
            constraint_iterations: 1,
            constraint_enforcement: IkConstraintEnforcement::AfterEachIteration,
        }
    }
}

impl IkSolveSettings {
    pub fn sanitized(self, defaults: Self) -> Self {
        Self {
            iterations: self.iterations.max(1),
            tolerance: if self.tolerance.is_finite() && self.tolerance > 0.0 {
                self.tolerance
            } else {
                defaults.tolerance
            },
            constraint_iterations: self.constraint_iterations.max(1),
            constraint_enforcement: self.constraint_enforcement,
        }
    }
}

#[derive(Clone, Copy, Debug, Reflect, PartialEq)]
pub struct IkWeight {
    pub overall: f32,
    pub position: f32,
    pub rotation: f32,
}

impl Default for IkWeight {
    fn default() -> Self {
        Self {
            overall: 1.0,
            position: 1.0,
            rotation: 1.0,
        }
    }
}

impl IkWeight {
    pub fn clamped(self) -> Self {
        Self {
            overall: self.overall.clamp(0.0, 1.0),
            position: self.position.clamp(0.0, 1.0),
            rotation: self.rotation.clamp(0.0, 1.0),
        }
    }

    pub fn position_factor(self) -> f32 {
        let clamped = self.clamped();
        clamped.overall * clamped.position
    }

    pub fn rotation_factor(self) -> f32 {
        let clamped = self.clamped();
        clamped.overall * clamped.rotation
    }
}

#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq)]
pub enum IkTargetSpace {
    #[default]
    World,
    LocalToEntity(Entity),
}

#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct IkGlobalSettings {
    pub default_solve: IkSolveSettings,
    pub preserve_initial_lengths: bool,
    pub log_invalid_chains_once: bool,
}

impl Default for IkGlobalSettings {
    fn default() -> Self {
        Self {
            default_solve: IkSolveSettings::default(),
            preserve_initial_lengths: true,
            log_invalid_chains_once: true,
        }
    }
}
