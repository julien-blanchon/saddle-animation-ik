# `saddle-animation-ik-lab`

Crate-local verification app for the shared `saddle-animation-ik` crate.

## Purpose

The lab keeps the shared crate testable without relying on project-level sandboxes. It shows four independent use cases in one scene:

- generic reach with a pole vector
- foot placement on stepped terrain with a root-offset hint
- a non-character crane arm using only the core chain solver
- look-at aiming with cone constraints

## Status

Working.

## How To Run

```bash
cargo run -p saddle-animation-ik-lab
```

Targeted E2E scenarios:

```bash
cargo run -p saddle-animation-ik-lab --features e2e -- ik_smoke
cargo run -p saddle-animation-ik-lab --features e2e -- ik_reach_target
cargo run -p saddle-animation-ik-lab --features e2e -- ik_foot_placement
cargo run -p saddle-animation-ik-lab --features e2e -- ik_crane_arm
cargo run -p saddle-animation-ik-lab --features e2e -- ik_constraint_debug
```

## BRP

The lab enables BRP through its default `dev` feature. The app-side listener respects `BRP_EXTRAS_PORT` and falls back to `15712`:

```bash
BRP_EXTRAS_PORT=15712 cargo run -p saddle-animation-ik-lab
uv run --active --project .codex/skills/bevy-brp/script brp status --port 15712
uv run --active --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name --port 15712
uv run --active --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/ik_lab.png --port 15712
```

## Findings

- the lab keeps all verification inside the shared crate tree
- the diagnostics resource gives a stable assertion surface for E2E and BRP
- debug drawing defaults on here so constraint and target guides are visible immediately
