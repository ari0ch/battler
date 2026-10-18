# Battler

A fantasy autobattler prototype where you program your units' behaviour with a tiny
Zachtronics-style instruction language, then watch the battle play out and debug your plan.

**Play the latest build:** https://ari0ch.github.io/battler/

Built in Rust with [Bevy](https://bevy.org). See [`docs/DESIGN.md`](docs/DESIGN.md) for
the design.

## Development

Requirements: a Rust stable toolchain (`rust-toolchain.toml` installs the WASM target)
and, for web builds, [trunk](https://trunkrs.dev) (`cargo install trunk`). On Linux,
Bevy also needs `libudev-dev`, `libwayland-dev` and `libxkbcommon-dev`.

```sh
cargo test -p battler-sim          # simulation tests
cargo dev                          # run natively (fast incremental builds)
cd crates/game && trunk serve      # run in the browser at http://127.0.0.1:8080
```

## Structure

| Path           | What it is                                                         |
| -------------- | ------------------------------------------------------------------ |
| `crates/sim`   | Deterministic battle simulation and interpreter. No dependencies. |
| `crates/game`  | Bevy client: rendering, program editor, replay.                    |
| `docs/`        | Design notes.                                                      |

Every push runs CI (fmt, clippy, tests). Every push to `main` deploys the web build to
GitHub Pages.
