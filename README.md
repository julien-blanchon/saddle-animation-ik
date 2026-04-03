# Saddle Animation IK

Reusable inverse-kinematics toolkit for articulated chains in Bevy. The crate focuses on runtime solving for generic chains instead of humanoid-only rigs, so the same API works for limbs, turrets, tentacles, cranes, and other authored transform chains.

The runtime stays project-agnostic. It does not depend on `game_core`, `Screen`, `GameSet`, or any project-specific type. Consumers wire it into their own schedules and feed it current transforms, targets, and optional helper components.

## Quick Start

```toml
[dependencies]
bevy = "0.18"
saddle-animation-ik = { git = "https://github.com/julien-blanchon/saddle-animation-ik" }
```

```rust,no_run
use bevy::prelude::*;
use saddle_animation_ik::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(IkPlugin::always_on(Update))
        .insert_resource(IkDebugSettings {
            enabled: true,
            ..default()
        })
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let root = commands
        .spawn((
            Name::new("Root Joint"),
            Transform::from_xyz(0.0, 0.0, 0.0),
            IkJoint::default(),
        ))
        .id();
    let mid = commands
        .spawn((
            Name::new("Mid Joint"),
            Transform::from_xyz(0.0, 1.0, 0.0),
            IkJoint::default(),
        ))
        .id();
    let tip = commands
        .spawn((
            Name::new("Tip Joint"),
            Transform::from_xyz(0.0, 2.0, 0.0),
            IkJoint::default(),
        ))
        .id();

    commands.spawn((
        Name::new("Reach Chain"),
        IkChain {
            joints: vec![root, mid, tip],
            solver: IkSolver::Fabrik,
            ..default()
        },
        IkTarget {
            position: Vec3::new(0.75, 1.7, 0.1),
            ..default()
        },
        PoleTarget {
            point: Vec3::new(1.0, 1.0, 1.5),
            ..default()
        },
    ));
}
```

## Public API

| Type | Purpose |
| --- | --- |
| `IkPlugin` | Registers the runtime with injectable activate, deactivate, and update schedules |
| `IkSystems` | Public ordering hooks: `Prepare`, `Solve`, `Apply`, `Debug` |
| `IkChain` | Ordered joint list, solver choice, per-chain solve settings, and blend weights |
| `IkJoint` | Local tip axis, pole axis, stiffness, and damping for one authored joint |
| `IkTarget` | Position or position+orientation goal with runtime weights and world/local interpretation |
| `IkTargetAnchor` | Follows another entity with a local offset so targets can stay attached to moving props |
| `PoleTarget` | Optional elbow or knee hint target |
| `IkConstraint` | Per-joint cone or hinge limit |
| `LookAtTarget` | Aim helper that resolves a point into a reach-orientation goal |
| `FootPlacement` | Surface-contact helper with normal alignment and a generic root-offset hint |
| `FullBodyIkRig` / `FullBodyIkRigState` | Root-aware coordinator that aggregates multiple chain root-offset hints into a single body/root translation |
| `IkChainState` | Per-chain diagnostics: status, error, target, effector, suggested root offset |
| `IkGlobalSettings` | Global defaults for iterations, tolerance, cache policy, and invalid-chain logging |
| `IkDebugSettings` / `IkDebugDraw` | Global and per-chain debug drawing controls |
| Pure solver API | `solve_chain`, `SolverChainState`, `ResolvedTarget`, `ResolvedPole`, `SolveResult` |

## Solver Support

Supported in v0.1:

- FABRIK for general chains
- CCD for shorter chains or alternate convergence behavior
- analytic two-bone solve for common limb cases
- per-chain solver selection
- optional position and orientation blending
- world-space and entity-local target interpretation
- pole vectors
- cone and hinge constraints
- runtime debug drawing
- root-aware multi-chain coordination for foot-placement or support-hand style full-body rigs

## Runtime Pipeline

The runtime is staged and orderable:

1. `Prepare`
2. `Solve`
3. `Apply`
4. `Debug`

`Prepare` reads the current authored pose, resolves targets, caches segment lengths, and produces a pure solver input. `Solve` runs FABRIK, CCD, or the two-bone solver. `Apply` writes corrected transforms back onto the authored entities. `Debug` draws chains, targets, reach radius, pole vectors, error lines, and constraint guides.

The prepare/apply path resolves current-frame hierarchy transforms from `Transform` data instead of trusting potentially stale `GlobalTransform` values from the previous propagation pass. That keeps the crate usable as a late correction layer on top of authored animation or other same-frame pose updates.

## Examples

| Example | Purpose | Run |
| --- | --- | --- |
| `basic` | Minimal moving-target reach with FABRIK and a pole vector | `cargo run -p saddle-animation-ik-example-basic` |
| `two_bone` | Analytic limb solve with a moving hint target | `cargo run -p saddle-animation-ik-example-two-bone` |
| `foot_placement` | Surface-contact target, foot normal alignment, and root-offset hint usage | `cargo run -p saddle-animation-ik-example-foot-placement` |
| `look_at` | Short aim chain with cone constraints | `cargo run -p saddle-animation-ik-example-look-at` |
| `support_hand` | Grip-point anchoring on a moving prop with orientation blending | `cargo run -p saddle-animation-ik-example-support-hand` |
| `multi_chain` | Mixed FABRIK and CCD stress preview with diagnostics plugins | `cargo run -p saddle-animation-ik-example-multi-chain` |

Every standalone example includes a live `saddle-pane` debug panel so solve iterations, tolerances, weights, and feature-specific knobs like ankle offset or reach distance can be tuned in real time.

## Crate-Local Lab

The richer verification app lives at `shared/animation/saddle-animation-ik/examples/lab`:

```bash
cargo run -p saddle-animation-ik-lab
```

Targeted E2E scenarios:

```bash
cargo run -p saddle-animation-ik-lab --features e2e -- ik_smoke
cargo run -p saddle-animation-ik-lab --features e2e -- ik_reach_target
cargo run -p saddle-animation-ik-lab --features e2e -- ik_foot_placement
cargo run -p saddle-animation-ik-lab --features e2e -- ik_constraint_debug
```

## Limitations and Non-Goals

Current limitations:

- the orientation goal is applied at the effector; v0.1 does not distribute a full orientation solve back through the chain
- constraints are enforced by projection during solve, not by a full optimization pass
- segment lengths are cached from the authored pose unless `IkGlobalSettings::preserve_initial_lengths` is disabled
- the root-offset helper publishes a suggested offset; the crate does not automatically decide which gameplay entity should consume it
- `FullBodyIkRig` is a pragmatic chain coordinator, not a dense single-pass FBIK optimizer

Intentional non-goals in v0.1:

- full-body IK
- retargeting
- physics-engine integration
- humanoid-only authoring assumptions

## More Docs

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
