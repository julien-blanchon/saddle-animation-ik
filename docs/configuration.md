# `saddle-animation-ik` Configuration

This document lists the public tuning surface for `saddle-animation-ik` in v0.1. Defaults shown here are the values returned by `Default::default()`.

## `IkChain`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `joints` | `Vec<Entity>` | `[]` | 2+ joints required | Ordered chain definition | Fewer than 2 joints marks the chain invalid |
| `enabled` | `bool` | `true` | `true` or `false` | Enables or disables solving | Disabled chains keep their authored pose |
| `solver` | `IkSolver` | `Fabrik` | `Fabrik`, `Ccd`, `TwoBone` | Selects the runtime solver | `TwoBone` on non-three-joint chains is rejected |
| `solve` | `IkSolveSettings` | see below | finite values only | Iteration and tolerance settings | Zero or invalid values are sanitized to global defaults |
| `weight` | `IkWeight` | all `1.0` | `0.0..=1.0` per field | Per-chain procedural blend | Zeroed weights can make the crate look inactive by design |

## `IkSolveSettings`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `iterations` | `usize` | `12` | `>= 1` | How many solver iterations to spend | Too low can leave visible error |
| `tolerance` | `f32` | `0.01` | `> 0` | Target distance considered "good enough" | Tiny values can waste work chasing imperceptible error |
| `constraint_iterations` | `usize` | `1` | `>= 1` | Reserved for future deeper projection loops; v0.1 keeps a single projection pass | Setting this high currently does not buy much |
| `constraint_enforcement` | `IkConstraintEnforcement` | `AfterEachIteration` | enum | Decides when the projection pass runs | `AfterSolve` can feel looser on heavily constrained chains |

## `IkWeight`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `overall` | `f32` | `1.0` | `0.0..=1.0` | Global multiplier | Zero preserves the authored pose |
| `position` | `f32` | `1.0` | `0.0..=1.0` | Position-goal influence | Too low leaves the effector visibly behind the target |
| `rotation` | `f32` | `1.0` | `0.0..=1.0` | Effector orientation influence | Too high can make the wrist or foot twist aggressively |

## `IkJoint`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `tip_axis` | `Vec3` | `Vec3::Y` | non-zero recommended | Local axis that points toward the next joint | Wrong axis makes the solved rotation appear 90 or 180 degrees off |
| `pole_axis` | `Vec3` | `Vec3::Z` | non-zero recommended | Local secondary axis for authored roll conventions | Mostly relevant when matching a rig's authored twist |
| `stiffness` | `f32` | `0.0` | `0.0..=1.0` | Blends the solved direction back toward the authored direction | Too high makes the chain feel unresponsive |
| `damping` | `f32` | `1.0` | `0.0..=1.0` | Blends solved positions toward the authored pose before final projection | Too low can look like the chain is lagging forever |

## `IkTarget`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `true` or `false` | Enables or disables the target | Disabled targets leave the chain at its authored pose unless a helper overrides it |
| `position` | `Vec3` | `Vec3::ZERO` | finite | Position goal | Invalid values produce an invalid solve |
| `orientation` | `Option<Quat>` | `None` | normalized quaternion recommended | Optional effector orientation goal | Non-normalized quaternions can cause unexpected effector twist |
| `space` | `IkTargetSpace` | `World` | enum | Interprets the position and orientation in world or entity-local space | Wrong space commonly creates large offsets |
| `weight` | `IkWeight` | all `1.0` | `0.0..=1.0` | Target-local blend | Zeroed position weight preserves the current effector position |

## `IkTargetAnchor`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `entity` | `Entity` | none | must exist | Follows another entity | Missing entity leaves the previous target values in effect |
| `translation_offset` | `Vec3` | `Vec3::ZERO` | finite | Local offset from the anchor entity | Large offsets can make the anchor look detached |
| `rotation_offset` | `Quat` | `Quat::IDENTITY` | normalized quaternion recommended | Additional orientation offset from the anchor entity | Bad offsets can make grip alignment look wrong |

## `PoleTarget`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `true` or `false` | Enables the pole hint | Disabled poles let elbows or knees pick their own bend plane |
| `point` | `Vec3` | `Vec3::ZERO` | finite | Hint location | If the point lies on the chain axis, it has little or no effect |
| `space` | `IkTargetSpace` | `World` | enum | World or entity-local interpretation | Wrong space often flips the bend direction |
| `weight` | `f32` | `1.0` | `0.0..=1.0` | Pole influence | High weight with a bad pole can make the chain look forced |

## `IkConstraint`

### `Cone`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `axis` | `Vec3` | authored tip axis fallback | non-zero recommended | Local axis defining the center of the cone | Wrong axis centers the cone in the wrong direction |
| `max_angle` | `f32` | none | `>= 0` radians | Maximum swing from the cone axis | Overly tight values can make the target unreachable very quickly |
| `strength` | `f32` | none | `0.0..=1.0` | How strongly the clamp is enforced | Weak values can let the chain drift beyond the authored limit |

### `Hinge`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `axis` | `Vec3` | none | non-zero | Local hinge axis | Wrong axis flips the hinge plane |
| `reference_axis` | `Vec3` | authored tip axis fallback | non-zero | Zero-angle direction inside the hinge plane | Wrong reference changes which side is "positive" |
| `min_angle` | `f32` | none | radians | Lower limit around the hinge axis | Swapped min/max clamps the chain into a tiny range |
| `max_angle` | `f32` | none | radians | Upper limit around the hinge axis | Wide limits make the hinge behave almost unconstrained |
| `strength` | `f32` | none | `0.0..=1.0` | Clamp strength | Weak values feel mushy |

## `LookAtTarget`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `true` or `false` | Enables the helper | Disabled helper does nothing |
| `point` | `Vec3` | `Vec3::ZERO` | finite | Aim point | Bad points create bad reach targets |
| `space` | `IkTargetSpace` | `World` | enum | World or local interpretation | Wrong space makes the chain aim somewhere surprising |
| `forward_axis` | `Vec3` | `Vec3::Z` | non-zero | Effector local axis that should face the target | Wrong axis makes the effector look sideways |
| `up_axis` | `Vec3` | `Vec3::Y` | non-zero | Secondary orientation axis | Wrong up axis produces odd roll |
| `reach_distance` | `Option<f32>` | `None` | `> 0` when set | Distance from root used to place the synthetic target | Too short can keep the chain folded up |
| `weight` | `IkWeight` | all `1.0` | `0.0..=1.0` | Blend for the generated target | Low position weight makes the aim chain look sluggish |

## `FootPlacement`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `true` or `false` | Enables the helper | Disabled helper does nothing |
| `contact_point` | `Vec3` | `Vec3::ZERO` | finite | Sampled surface point | Bad sampling data gives bad placement |
| `contact_normal` | `Vec3` | `Vec3::Y` | non-zero recommended | Sampled surface normal | Zero normals fall back to world up |
| `space` | `IkTargetSpace` | `World` | enum | World or local interpretation | Wrong space offsets the foot |
| `ankle_offset` | `f32` | `0.02` | finite | Lifts the target off the contact point | Too low can visually sink into the floor |
| `foot_up_axis` | `Vec3` | `Vec3::Y` | non-zero | Local axis aligned to the contact normal | Wrong axis makes the foot roll incorrectly |
| `foot_forward_axis` | `Vec3` | `Vec3::Z` | non-zero | Local forward axis preserved across the surface | Wrong axis can make the foot point sideways |
| `normal_blend` | `f32` | `1.0` | `0.0..=1.0` | Blend toward the sampled surface normal | Full alignment on noisy normals can jitter |
| `root_offset_hint` | `Option<RootOffsetHint>` | `None` | optional | Computes a suggested root translation | Expecting automatic pelvis motion without consuming the hint is a common misunderstanding |

## `RootOffsetHint`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `axis` | `Vec3` | `Vec3::Y` | non-zero | Axis along which to suggest root motion | Wrong axis suggests movement in the wrong direction |
| `max_distance` | `f32` | `0.35` | `>= 0` | Clamp on the hint magnitude | Tiny values can hide useful hints |
| `weight` | `f32` | `1.0` | `0.0..=1.0` | Blend of the final hint | Zero disables the hint |

## `IkGlobalSettings`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `default_solve` | `IkSolveSettings` | default solve settings | see above | Fallback when per-chain settings are invalid | Unexpected sanitization if chain settings contain zero or NaN values |
| `preserve_initial_lengths` | `bool` | `true` | `true` or `false` | Keeps segment lengths fixed from the authored pose | Turning this off on noisy rigs can make reach drift frame to frame |
| `log_invalid_chains_once` | `bool` | `true` | `true` or `false` | Limits invalid-chain log spam | Disabling it can spam logs on broken rigs |

## `IkDebugSettings`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `enabled` | `bool` | `false` | `true` or `false` | Master debug toggle | Leaving it on in normal gameplay adds avoidable debug cost |
| `draw_targets` | `bool` | `true` | `true` or `false` | Draw target spheres | None |
| `draw_pole_vectors` | `bool` | `true` | `true` or `false` | Draw pole lines and markers | None |
| `draw_reach_radius` | `bool` | `true` | `true` or `false` | Draw a reach circle around the root | None |
| `draw_error_lines` | `bool` | `true` | `true` or `false` | Draw the line from effector to target | None |
| `draw_constraints` | `bool` | `true` | `true` or `false` | Draw cone and hinge guides | None |

## `IkDebugDraw`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `true` or `false` | Per-chain opt-in under the global debug toggle | Forgetting this component makes a chain disappear from debug view |
| `color` | `Color` | cyan-like | any Bevy color | Debug color for the chain | Very dark colors can become unreadable |
| `joint_radius` | `f32` | `0.05` | `> 0` | Debug sphere size | Oversized values clutter dense scenes |
| `draw_constraints` | `bool` | `true` | `true` or `false` | Per-chain constraint guide toggle | None |
