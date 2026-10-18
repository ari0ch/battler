# Battler: notes for Claude

A prototype autobattler in which units run small Zachtronics-style programs. It is also a
project for learning Rust in depth, so prefer idiomatic, well-explained Rust over clever
shortcuts, and point out interesting language details when they come up.

Design context lives in `docs/DESIGN.md`. Keep it updated when decisions are made.

## Layout

- `crates/sim`: `battler-sim`, the deterministic battle simulation and interpreter.
  **No dependencies.** Keep it that way unless there's a strong reason, and discuss first.
- `crates/game`: `battler`, the Bevy 0.20 client. Rendering, input, editor, replay.
  It reads sim state; game rules never live here.
- `.github/workflows/ci.yml`: fmt, clippy (`-D warnings`) and tests on every push.
- `.github/workflows/deploy.yml`: trunk WASM build, deployed to GitHub Pages on `main`.
  The live build is at https://ari0ch.github.io/battler/

## Sim rules (determinism)

- Integers only in game logic; no `f32`/`f64`.
- No `HashMap`/`HashSet` iteration in logic; use `Vec` or `BTreeMap` with stable order.
- No wall-clock time and no OS randomness. Time is the tick counter; any randomness
  comes from a seeded RNG stored in the `Battle`.
- Units act in ascending `UnitId` order. Ties are broken by id.
- Every observable change emits an `Event`; the event log is the replay.
- Add a test for each new rule, including a determinism check where relevant.

## Commands

- `cargo test -p battler-sim`: fast sim tests.
- `cargo dev`: run the game natively with dynamic linking (alias in `.cargo/config.toml`).
- `cd crates/game && trunk serve`: run the web build locally.
- `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` before committing.

## Bevy

Pinned to 0.20 with only the `2d` and `ui` feature collections, to keep the WASM build
small. Check the docs for the pinned version, not older examples, because APIs change
between releases. Upgrades are deliberate and follow the official migration guide.

## Working from Claude's cloud workspace

crates.io is not reachable from that workspace, so the Bevy crate can't be built there.
The sim crate can be tested by copying it outside the workspace (it has no
dependencies). Otherwise, push to a branch and use the CI results.
