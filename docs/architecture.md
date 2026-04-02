# `saddle-animation-ik` Architecture

## Goals

`saddle-animation-ik` is a generic runtime IK layer for Bevy transform chains. The design assumes that a chain already has a sensible authored pose each frame, then applies procedural correction on top of that pose. This makes the crate suitable for gameplay correction, not just static solver demos.

The main design choices are:

- the chain description is the core abstraction
- the solver layer stays pure-Rust and Bevy-free except for math types
- ECS glue is thin and mostly responsible for gathering inputs and writing transforms back
- helper components such as `FootPlacement` and `LookAtTarget` resolve into the same target model as `IkTarget`

## Chain Model

An `IkChain` is an ordered `Vec<Entity>` of joint entities. The crate does not assume a humanoid skeleton, a specific rig import path, or a specific mesh setup.

Each joint may carry:

- `IkJoint`
- `IkConstraint`
- an authored `Transform` and `GlobalTransform`

The ordered list defines the kinematic chain even if the scene hierarchy has extra helpers between joints. In the common case, the listed joints are direct parent-child entities, which also makes the crate immediately compatible with Bevy skinning because the solver writes back onto the same transform entities that a `SkinnedMesh` already follows.

## Solve Pipeline

### 1. Prepare

`Prepare` reads the current authored pose and resolves:

- world-space joint positions
- authored world rotations
- cached segment lengths
- the active target
- the active pole target
- optional foot-placement root-offset hint

The runtime computes those world-space inputs from the current `Transform` hierarchy each frame. It does not rely on last-frame `GlobalTransform` values, which avoids a one-frame lag when authored animation, moving props, or procedural pose edits run earlier in the same frame.

Target resolution priority is:

1. `IkTarget`
2. `IkTargetAnchor` adjusts that target from another entity
3. `LookAtTarget` may replace the target with a reach-orientation goal
4. `FootPlacement` may replace the target with a surface-contact goal

This order is intentional: helpers are thin wrappers over the same target contract instead of separate solve paths.

### 2. Solve

The pure solver layer receives a `SolverChainState`, a `ResolvedTarget`, an optional `ResolvedPole`, and sanitized solve settings.

Supported solvers:

- `IkSolver::Fabrik`
- `IkSolver::Ccd`
- `IkSolver::TwoBone`

Solver selection is per-chain.

### 3. Constraint Projection

`saddle-animation-ik` uses a readable, conservative constraint strategy:

- the core position solver runs first
- pole adjustment runs on the intermediate position result
- a projection pass walks the chain from root to tip
- each segment direction is clamped by the authored constraint and blended back toward the authored direction according to per-joint stiffness

This keeps the behavior predictable, avoids NaNs on bad data, and makes the failure mode easy to inspect. It is not a global optimum solver. When constraints conflict with the target, the chain settles into a valid constrained pose and reports the remaining error.

### 4. Apply

`Apply` reconstructs local transforms from the solved world transforms using the actual Bevy parent global transform of each joint entity. That means:

- direct parent-child joint hierarchies work naturally
- extra helper parents can still work
- the same runtime can correct plain transform chains, GLTF skeletons, and mechanical rigs

## Rotation Model

The solver is primarily positional. Rotations are produced after the positions settle:

- each joint rotation starts from the authored world rotation
- the joint tip axis from `IkJoint::tip_axis` is aligned toward the next solved joint position
- the effector optionally blends toward the target orientation

This preserves authored twist better than solving from a neutral identity pose every frame. It also means the orientation goal in v0.1 is an effector-alignment feature, not a distributed orientation constraint across the whole chain.

## Local vs World Targets

`IkTarget`, `PoleTarget`, `LookAtTarget`, and `FootPlacement` all support `IkTargetSpace`:

- `World`
- `LocalToEntity(Entity)`

`LocalToEntity` is resolved during `Prepare` against that entity's current `GlobalTransform`. This lets consumers keep targets attached to handles, weapons, or moving platforms without custom solver code.

## Foot Placement Model

`FootPlacement` does not own terrain queries. Instead it accepts:

- a contact point
- a contact normal
- ankle offset
- alignment axes
- an optional `RootOffsetHint`

This is deliberate. Terrain sampling is gameplay-specific and often tied to physics or bespoke traces. The crate only converts a sampled contact into an IK target plus an optional root-offset suggestion. Consumers can feed it data from raycasts, sweeps, navmesh sampling, animation events, or offline authored contacts.

## Caching Strategy

The runtime caches segment lengths in `IkChainCache`.

Default behavior:

- lengths are captured from the authored pose once
- later solves reuse those cached values

If a consumer truly wants runtime-changing lengths, `IkGlobalSettings::preserve_initial_lengths = false` tells `Prepare` to re-measure lengths from the current pose every frame.

This tradeoff keeps the default cost predictable and avoids silently changing limb reach because of temporary animation compression or jitter.

## Performance Notes

The main per-frame costs are:

- resolving joint entity transforms
- solver iteration count
- constraint projection count
- debug drawing

Practical tuning advice:

- use FABRIK for general chains and longer limbs
- use CCD when the chain is short and you want a different feel
- use `TwoBone` for common limb rigs when the chain really is three joints
- keep debug drawing off outside labs or tooling
- keep iterations low unless the target is visibly undershooting

`multi_chain` demonstrates a mixed FABRIK/CCD load case with Bevy diagnostics plugins enabled.
