mod components;
mod config;
mod constraints;
mod debug;
mod math;
mod solver;
mod systems;

pub use components::{
    FootPlacement, FullBodyIkChain, FullBodyIkRig, FullBodyIkRigState, IkChain, IkChainState,
    IkDebugDraw, IkJoint, IkTarget, IkTargetAnchor, LookAtTarget, PoleTarget, RootOffsetHint,
};
pub use config::{
    IkConstraintEnforcement, IkGlobalSettings, IkSolveSettings, IkSolveStatus, IkSolver,
    IkTargetSpace, IkWeight,
};
pub use constraints::IkConstraint;
pub use debug::IkDebugSettings;
pub use math::{
    align_axis_rotation, compute_root_offset_hint, orientation_from_axes, project_on_plane,
    safe_normalize,
};
pub use solver::{
    ResolvedPole, ResolvedTarget, SolveResult, SolverChainState, SolverJointState, solve_chain,
};

use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    gizmos::{config::DefaultGizmoConfigGroup, gizmos::GizmoStorage},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum IkSystems {
    Prepare,
    Solve,
    Apply,
    Debug,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct IkPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl IkPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Default for IkPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for IkPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        app.init_resource::<systems::IkRuntimeState>()
            .init_resource::<IkGlobalSettings>()
            .init_resource::<IkDebugSettings>()
            .register_type::<FootPlacement>()
            .register_type::<FullBodyIkChain>()
            .register_type::<FullBodyIkRig>()
            .register_type::<FullBodyIkRigState>()
            .register_type::<IkChain>()
            .register_type::<IkChainState>()
            .register_type::<IkConstraint>()
            .register_type::<IkDebugDraw>()
            .register_type::<IkDebugSettings>()
            .register_type::<IkGlobalSettings>()
            .register_type::<IkJoint>()
            .register_type::<IkSolveSettings>()
            .register_type::<IkSolver>()
            .register_type::<IkTarget>()
            .register_type::<IkTargetAnchor>()
            .register_type::<IkTargetSpace>()
            .register_type::<IkWeight>()
            .register_type::<LookAtTarget>()
            .register_type::<PoleTarget>()
            .register_type::<RootOffsetHint>()
            .add_systems(self.activate_schedule, systems::activate_runtime)
            .add_systems(self.deactivate_schedule, systems::deactivate_runtime)
            .configure_sets(
                self.update_schedule,
                (
                    IkSystems::Prepare,
                    IkSystems::Solve,
                    IkSystems::Apply,
                    IkSystems::Debug,
                )
                    .chain(),
            )
            .add_systems(
                self.update_schedule,
                (
                    (
                        systems::ensure_chain_state,
                        systems::ensure_full_body_rig_state,
                        systems::capture_full_body_authored_roots,
                        systems::prepare_chains,
                    )
                        .chain()
                        .in_set(IkSystems::Prepare),
                    systems::solve_chains.in_set(IkSystems::Solve),
                    (systems::apply_chains, systems::apply_full_body_rigs)
                        .chain()
                        .in_set(IkSystems::Apply),
                )
                    .run_if(systems::runtime_is_active),
            );

        app.add_systems(
            self.update_schedule,
            debug::draw_debug_gizmos
                .in_set(IkSystems::Debug)
                .run_if(systems::runtime_is_active)
                .run_if(resource_exists::<GizmoStorage<DefaultGizmoConfigGroup, ()>>),
        );
    }
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
