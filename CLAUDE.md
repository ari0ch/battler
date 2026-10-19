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
- `crates/sim/src/run/`: the run. `Lobby` (eight players, shared `Pool`, matchmaking,
  round resolution, knock-outs), `Player` (one board: shop, bench, merges, economy,
  per-round `BattleSetup` history for replays) and `bot` (computer players). **Every
  change to a player goes through `Lobby::apply(PlayerId, Command)`**, for humans and
  bots alike; bots never get a shortcut. Monsters for PvE rounds live in `monster.rs`.
  Tuning numbers live in `run/economy.rs`; class/character stats in `class.rs`; synergy
  rules in `synergy.rs`; monster and boss stats in `monster.rs`.
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
- `cargo run -p battler-sim --example balance --release`: balance harness (character and
  class win rates in random fights; full lobbies: length, PvE win rates). Run it after
  changing numbers.
- `cargo dev`: run the game natively with dynamic linking (alias in `.cargo/config.toml`).
- `cd crates/game && trunk serve`: run the web build locally.
- `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` before committing.

## Game crate architecture

- `state.rs`: `AppState` (MainMenu, InRun, GameOver, Replay) with the `RunPhase` sub-state
  (Planning, Battle, Result), plus shared resources. Screens spawn with
  `DespawnOnExit(..)` so they clean up on state change.
- `ui.rs`: everything is Bevy UI on a fixed 1280x720 stage scaled with `UiScale`.
  Clickables are headless `bevy::ui_widgets::Button`s tagged `Clickable(UiAction)`; one
  global `Activate` observer queues actions. Drag and drop: `DragSource` / `DropTarget`
  plus picking observers. **Import `bevy::ui_widgets::Button` explicitly**: the prelude's
  `Button` and `Interaction` are deprecated in 0.20 and fail clippy.
- `actions.rs`: the only place UI input changes the lobby. The human is player 0
  (`state::ME`); `state::LobbyExt` adds `me()` and `act(Command)` to `Lobby`.
- `planning.rs`: the one-screen planning layout. `editor.rs`: the program column (cards +
  palette) and editing logic. `cards.rs`: all block/character/synergy wording and tooltips.
- Tooltips: add `Hovered::default()` plus a `ui::Tooltip` to any entity; one shared
  floating panel shows it.
- `sprites.rs`: the 18 character and 8 monster sprites (32x32 PNGs in
  `crates/game/assets/sprites`), embedded with `include_bytes!` and decoded once into
  the `Sprites` resource. Draw them with `sprite_node` at whole-number scales (2x on the
  board, `battle_scale` for bosses) so pixels stay crisp.
- `kit.rs`: the fantasy UI kit (`assets/ui`) and the Pixelify Sans font, as **constant
  handles** (`kit::img::PANEL_WOOD`, `kit::FONT`) made with `uuid_handle!` and inserted
  at startup, so builders use them without a resource. Helpers: `sliced` (9-slice,
  corners at 2x), `tiled`, `icon`, `spawn_bar`, `wood_panel` (light text),
  `parchment_panel` (use `color::INK*` text on it). Buttons take a `ui::Skin` (wood, gold,
  pressed, parchment, flat). Regenerate art with `tools/art/`.
- Boards are drawn rotated from sim coordinates: lanes (`y`) across the screen, depth
  (`x`) up the screen with the enemy half above. Planning shows only the player's half.
- Screens rebuild from state whenever the `Rev` resource is bumped (every applied
  action, every state entry). Fast-changing bits (timer, toasts, battle animation)
  update in place via marker components, so a rebuild never interrupts a drag.
- Text uses `ui::text` / `ui::heading` / `ui::outlined` (the kit font). Keep it ASCII.
- Bevy 0.20 source is the best API reference: `git clone --depth 1 --branch v0.20.0
  --filter=blob:none --sparse https://github.com/bevyengine/bevy` and sparse-checkout
  `examples/ui` and the crate sources you need.

## Bevy

Pinned to 0.20 with only the `2d` and `ui` feature collections, to keep the WASM build
small. Check the docs for the pinned version, not older examples, because APIs change
between releases. Upgrades are deliberate and follow the official migration guide.

## Working from Claude's cloud workspace

crates.io is not reachable from that workspace, so the Bevy crate can't be built there.
The sim crate can be tested by copying it outside the workspace (it has no
dependencies). Otherwise, push to a branch and use the CI results.
