# `saddle-animation-ik` Configuration

This document lists the public tuning surface for `saddle-animation-ik` in v0.1. Defaults shown here are the values returned by `Default::default()`.

The core runtime is registered by `IkPlugin`. Character-oriented helpers such as look-at, foot placement, and multi-chain root coordination live under `saddle_animation_ik::helpers` and require `helpers::IkRigHelpersPlugin`.

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

## `IkChainState`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `status` | `IkSolveStatus` | `Disabled` | Last solve outcome for the chain | `InvalidChain` indicates bad input or missing joints |
| `cache_ready` | `bool` | `false` | Whether segment-length cache has been established | Useful for debugging first-frame setup |
| `last_error` | `f32` | `0.0` | Distance from effector to the resolved target after solve | Larger values usually mean low iterations or unreachable targets |
| `unreachable` | `bool` | `false` | Whether the target exceeded the chain reach | `true` is expected for overshoot cases |
| `target_position` | `Vec3` | `Vec3::ZERO` | Final world-space target used by the solver | Includes target anchors and helper overrides |
| `effector_position` | `Vec3` | `Vec3::ZERO` | Final world-space effector position after solve | Compare with `target_position` to inspect miss distance |
| `total_length` | `f32` | `0.0` | Cached total chain length used for solve | Respects `IkGlobalSettings::preserve_initial_lengths` |

## `helpers::LookAtTarget`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `true` or `false` | Enables the helper | Disabled helper does nothing |
| `point` | `Vec3` | `Vec3::ZERO` | finite | Aim point | Bad points create bad reach targets |
| `space` | `IkTargetSpace` | `World` | enum | World or local interpretation | Wrong space makes the chain aim somewhere surprising |
| `forward_axis` | `Vec3` | `Vec3::Z` | non-zero | Effector local axis that should face the target | Wrong axis makes the effector look sideways |
| `up_axis` | `Vec3` | `Vec3::Y` | non-zero | Secondary orientation axis | Wrong up axis produces odd roll |
| `reach_distance` | `Option<f32>` | `None` | `> 0` when set | Distance from root used to place the synthetic target | Too short can keep the chain folded up |
| `weight` | `IkWeight` | all `1.0` | `0.0..=1.0` | Blend for the generated target | Low position weight makes the aim chain look sluggish |

## `helpers::FootPlacement`

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
| `root_offset_hint` | `Option<helpers::RootOffsetHint>` | `None` | optional | Computes a suggested root translation | Expecting automatic pelvis motion without consuming the hint is a common misunderstanding |

## `helpers::RootOffsetHint`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `axis` | `Vec3` | `Vec3::Y` | non-zero | Axis along which to suggest root motion | Wrong axis suggests movement in the wrong direction |
| `max_distance` | `f32` | `0.35` | `>= 0` | Clamp on the hint magnitude | Tiny values can hide useful hints |
| `weight` | `f32` | `1.0` | `0.0..=1.0` | Blend of the final hint | Zero disables the hint |

## `helpers::IkRootOffsetState`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `suggested_root_offset` | `Vec3` | `Vec3::ZERO` | Helper-produced root translation suggestion for one chain | `helpers::FullBodyIkRig` aggregates these values across multiple chains |

## `helpers::FullBodyIkRig`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `true` or `false` | Enables the rig coordinator | Disabled rigs stop applying coordinated root motion |
| `root_entity` | `Entity` | required | must exist | Shared body/root entity that receives the aggregated offset | Missing roots leave the state populated but nothing moves |
| `chains` | `Vec<helpers::FullBodyIkChain>` | `[]` | any length | Chains whose `helpers::IkRootOffsetState` values feed the coordinator | Forgetting to register a chain makes the rig look inert |
| `root_axis` | `Vec3` | `Vec3::Y` | non-zero recommended | Axis along which the final root motion is projected | Wrong axis makes the body shift in the wrong direction |
| `max_root_offset` | `f32` | `0.45` | `>= 0` | Clamp for the aggregated root offset magnitude | Very low values hide useful foot-placement correction |
| `root_blend` | `f32` | `1.0` | `0.0..=1.0` | Blend factor for the final root offset | Low values make the full-body response feel mushy |
| `apply_translation` | `bool` | `true` | `true` or `false` | Applies the result to the root transform automatically | Turning this off requires the consumer to read `helpers::FullBodyIkRigState` manually |

## `helpers::FullBodyIkChain`

| Field | Type | Default | Valid Range | Effect | Common Failure Mode |
| --- | --- | --- | --- | --- | --- |
| `chain_entity` | `Entity` | required | must exist | Source chain contributing a root-offset hint | Missing chain entities silently reduce the rig response |
| `influence` | `f32` | `1.0` | `> 0` recommended | Relative weight of that chain in the final average | Large mismatches can make one limb dominate the full-body solve |

## `helpers::FullBodyIkRigState`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `authored_root_translation` | `Vec3` | `Vec3::ZERO` | Last authored root translation captured before rig application | Lets the rig apply an offset without permanently drifting the root |
| `combined_root_offset` | `Vec3` | `Vec3::ZERO` | Aggregated root translation applied this frame | Good E2E/debug surface for pelvis/body adjustment |
| `active_chains` | `usize` | `0` | Number of contributing chains this frame | Useful for diagnosing missing chain links |
| `max_chain_error` | `f32` | `0.0` | Largest contributing chain error this frame | Helps spot when one limb is forcing a bad overall body correction |

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
