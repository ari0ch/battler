# Battler: design notes

A living document. Decisions move from **Open questions** to **Decided** as we settle them.

## Pitch

A fantasy autobattler where you don't just pick and place units: you **program** them.
Each unit carries a tiny, Zachtronics-style program that governs its behaviour in battle.
The skill lies in planning, and in reading why a plan went wrong.

## Pillars

1. **Small language, deep puzzles.** A handful of blocks and scarce program nodes.
   Depth comes from combining them, not from a long block list.
2. **Readable failure.** You must always be able to see *why* a battle went wrong, and tell
   a bad program apart from bad luck. Randomness is small, bounded, and visible in battle.
3. **Reproducible battles.** Battles may include randomness, but it comes from a seed
   stored with the battle. The same boards, programs and seed always replay identically,
   which powers the post-game replay, debugging, tests, and any future PvP.

## Decided

### Technology

- **Engine and language:** Rust with Bevy (pinned at 0.20). This is also a Rust learning project.
- **Architecture:** a pure-Rust `battler-sim` crate with no engine dependencies holds all
  game rules: the battle, the interpreter, and later the shop and economy. The Bevy
  `battler` crate only renders state, takes input, and hosts the editor and replay UI.
- **Primary target:** the web (WASM, WebGL2), deployed to GitHub Pages on every push to
  `main`, for easy playtest links. Desktop is used for the development loop only.

### Programming

- **Editor:** Scratch-style drag and drop. Each unit has a stack of **nodes**.
- **A node is a rule card:** an optional condition plus one action
  (`[enemy adjacent?] → [ATTACK]`). Each tick the unit checks its cards top to bottom and
  runs the first whose condition is true. Card order is part of the puzzle.
- **Node count grows with unit level:** level 1 has 2 nodes, level 2 has 3, level 3 has 4.
- **Programming happens in the planning phase** between battles. Rounds are time-limited.

### Units and board

- **One copy of each unit type on the board.** Further copies sit inactive on the bench.
- **Limited bench slots** (TFT-style), so players can only chase a few upgrades at once.
- **Levelling:** three copies merge into the next level (3 for level 2, 9 for level 3).
  The merged unit is the one on the board, so **its program carries over** and gains an
  empty node to fill. If no copy is on the board, the merged unit starts unprogrammed.

### Economy

- **TFT-style:** gold per round, a win/loss bonus, and interest on unspent gold.
  Starting values to tune: 5 base gold per round; +1 interest per 10 banked, up to +5;
  streak bonuses.

### Analysis

- **Post-game replay** of any battle from the run, for analysis rather than mid-run
  rematches. Cheap to store: the boards, programs and seed, not a recording.

## Open questions

1. **Long-term structure:** async PvP (Super Auto Pets style), a single-player roguelike
   run against AI boards, or both? This affects how the economy is tuned.
2. **Where does randomness appear?** Candidates: miss/dodge chances, damage ranges,
   crits. Keep it small enough that close fights can swing, but a clearly better program
   still wins.
3. **How long is a planning phase?** It has to fit shopping and programming. Possibly
   longer than TFT's, or scaling with the number of units.
4. **Bench size and board size.** Does the board limit grow with player level, as in TFT?

## Instruction language (sketch)

The current sim has a placeholder set (`Wait`, `Advance`, `Attack`, `Guard`) as a looping
sequence. It will be replaced by rule cards. Ideas for blocks:

- **Conditions:** `enemy adjacent`, `hp below X%`, `ally hurt`, `outnumbered`,
  `signal received`.
- **Actions:** `ADVANCE`, `ATTACK`, `GUARD`, `RETREAT`, plus unit-specific ones.
- **Blocks come from units:** each unit type contributes its own blocks (a Knight brings
  `TAUNT`, a Mage brings `FIREBALL`), alongside a shared basic set. Traits/synergies could
  unlock team-wide blocks such as `SEND SIGNAL`, which is where coordination between
  units comes from. Buying a unit chooses vocabulary as well as stats.

Example (a level 3 Knight with 4 nodes):

```
1: [hp < 30%?]         → [GUARD]
2: [ally hurt?]        → [TAUNT]
3: [enemy adjacent?]   → [ATTACK]
4: [always]            → [ADVANCE]
```

## Prototype scope

Goal: prove the program → watch → analyse loop is fun, then add the shop.

- **Phase 1:** rule-card programs, 3–4 unit types, around 8 blocks, a small grid,
  hand-designed enemy boards, a drag-and-drop editor (`bevy_egui` is the likely quick
  path), and a replay showing which card each unit is running.
- **Phase 2:** shop, bench, merging, economy, timed rounds, post-game replay.
