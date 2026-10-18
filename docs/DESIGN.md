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
  game rules: the battle, the interpreter, and the run (shop, pool, economy). The Bevy
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

### Run structure

- **Single-player first, against AI boards.** AI opponents use the same shop and program
  rules as the player, drawing on pre-written programs per class, and get stronger each
  round. Async PvP can come later by swapping AI boards for snapshots of real players.
- **Player health:** 100 HP. A loss costs **2 + the total level of the enemy units still
  alive**, so a narrow loss hurts less than a wipe. A draw costs nothing.
- **The run ends** at 0 HP (defeat) or after winning through round 15 (victory).
- **Board size grows by round:** 3 units (rounds 1–2), 4 (3–5), 5 (6–9), 6 (10+).
  Six is also the number of classes, which the one-per-type rule caps anyway.
- **Battlefield:** 8×5 grid. The player places units in the left four columns; the
  enemy board occupies the right four.

### Scenes

Main menu → **run loop** (Planning → Battle → Result, repeated each round) → Game over →
Run summary → Main menu.

- **Main menu:** new run, how to play, settings.
- **Planning (timed):** shop, bench, board placement, and the card editor.
  60 seconds, or 75 once the player has 5+ units. A **Ready** button ends it early.
- **Battle:** watch the fight, at 1×/2×/4× speed or skipped to the end.
- **Result:** win/loss, HP lost, and a gold breakdown for next round.
- **Game over:** victory or defeat, rounds survived, then the run summary.
- **Run summary:** stats per round and a replay of any battle from its seed.

### Shop and bench

- 5 offers per round. **Buy 3 gold**, **reroll 2 gold**, **sell** for the purchase price
  of the copies inside (3 per copy).
- **Bench: 4 slots.** Buying a third copy merges immediately, freeing slots.
- **Shared pool:** about 12 copies of each class, drawn from the run's seeded RNG.
- **Programs persist between rounds.** Click any unit (board or bench) to edit its cards.

### Economy

- Start with **10 gold**.
- Each round: **5 base**, **+1 for a win**, **interest +1 per 10 gold banked** (max +5),
  **streak bonus** +1/+2/+3 at 2/4/6 consecutive wins *or* losses.

### Battle readability

The renderer turns each tick's events into short animations over the tick's duration.
In priority order:

1. **Card fired:** a badge above the acting unit showing the action; a side panel lists
   every unit's cards with the running one highlighted.
2. **Health bars, level pips,** and a team-coloured ring under each unit.
3. **Floating numbers:** damage, larger yellow crits with "!", green heals, "Blocked".
4. **Smooth movement and attacks:** glide between cells, melee lunges, projectiles.
5. **Status icons:** shield, frenzy, immunity; taunt draws a line to the Knight.
6. **Signals** pulse between allies.

### Analysis

- **Post-game replay** of any battle from the run, for analysis rather than mid-run
  rematches. Cheap to store: the boards, programs and seed, not a recording.

### Presentation tech

- **Bevy states:** `AppState` (Main menu, In run, Game over) with a nested `RunPhase`
  (Planning, Battle, Result). Entities are scoped to the state that created them and
  despawned on exit.
- **Run, shop, pool and economy rules live in `battler-sim`** (a `run` module with its own
  seeded RNG and tests). The game crate only displays them and sends player commands.
- **UI uses Bevy's own UI and picking** (drag events included), not `bevy_egui`.
- **Art later.** Pixel art is the likely direction; until then, clean placeholder shapes
  and icons.

## Open questions

1. **More randomness?** Crits are implemented (see below). Dodges or damage ranges could
   follow if fights feel too predictable.
2. **Traits/synergies.** Signals are currently open to every unit; they could become a
   trait unlock later.
3. **AI difficulty curve:** how quickly AI boards gain levels and units.
4. **Art direction and sourcing** for pixel art.

## Rules (implemented in `battler-sim`)

- **Evaluation:** each tick, units act in id order. A unit checks its cards top to bottom
  and runs the first whose condition holds **and** whose action can be carried out. If the
  action can't run (no target in range, ability on cooldown, ally already at full HP...),
  it **falls through** to the next card.
- **Locked final rule:** if no card fires, the unit attacks the nearest enemy, advancing
  toward it when out of range. It is not one of the unit's nodes and can't be removed, so
  units never stand idle.
- **Targeting:** actions with a target have a dropdown: nearest, lowest HP, highest HP,
  strongest, ranged. Ties break by distance, then unit id. Targeting the lowest-HP
  **enemy** is Ranger-only; for allies, "lowest HP" means most hurt by percentage.
- **Taunt overrides targeting:** a taunted unit targets its taunter with every
  enemy-targeted action, whatever its dropdown says.
- **Cooldowns:** each signature ability has its own cooldown. Abilities start ready.
- **Signals:** sent on one tick, visible to the whole team on the next tick only.
- **Defences:** Guard and Shield Wall halve damage (rounding up; they don't stack).
  Sanctuary blocks all damage.
- **Crits:** each hit has a 15% chance to deal 150% damage, from the battle's seeded RNG.
  Crit chance can be set to 0 per battle (useful for tests, or a no-luck mode).
- **Levels:** HP and attack scale ×1.8 per level (100% / 180% / 324%); range doesn't.

## Classes

| Class | HP | Attack | Range | Signature blocks |
|---|---|---|---|---|
| Knight | 600 | 40 | 1 | Taunt, Shield Wall |
| Ranger | 350 | 55 | 3 | Aimed Shot, Kite |
| Mage | 350 | 45 | 3 | Fireball, Blink |
| Cleric | 400 | 25 | 2 | Heal, Sanctuary |
| Rogue | 400 | 60 | 1 | Backstab, Shadowstep |
| Berserker | 550 | 50 | 1 | Frenzy, Cleave |

(Level 1 stats. All numbers are first guesses for tuning.)

## Block reference

**Conditions** (shared): always · enemy adjacent · enemy in range · my HP below 25/50/75% ·
ally HP below 25/50/75% · outnumbered (more enemies than allies within 2 cells) ·
ability ready · every N ticks (1–9) · signal received (red/blue).

**Shared actions:**

| Action | Effect | Can't run when |
|---|---|---|
| Advance ▾ | step toward target | target already in range, or blocked |
| Attack ▾ | basic attack | target out of range |
| Guard | half damage until my next turn | never |
| Retreat | step away from the nearest enemy | nowhere further to go |
| Hold | do nothing on purpose | never |
| Protect Ally ▾ | move next to an ally | already adjacent, or blocked |
| Send Signal | red or blue, seen by the team next tick | never |

**Signature actions:**

| Action | Effect | Cooldown |
|---|---|---|
| Taunt | enemies within 2 cells must target me for 3 ticks | 6 |
| Shield Wall | me and adjacent allies take half damage for 2 ticks | 6 |
| Aimed Shot ▾ | 2× damage, range +1 | 4 |
| Kite ▾ | attack, then step away from the nearest enemy | 2 |
| Fireball ▾ | hits target, plus half damage to enemies next to it | 5 |
| Blink | teleport up to 3 cells to the spot furthest from enemies | 6 |
| Heal ▾ | restore 4× attack HP to an ally in range +1 | 3 |
| Sanctuary ▾ | ally in range +1 is immune to damage for 2 ticks | 8 |
| Backstab ▾ | 3× damage on an adjacent enemy that's next to one of my allies | 4 |
| Shadowstep ▾ | teleport next to an enemy within 4 cells, on its far side | 5 |
| Frenzy | for 4 ticks, basic attacks strike twice | 8 |
| Cleave | hit every adjacent enemy | 3 |

▾ = has a target dropdown. Frenzy is simplified from "attack faster as HP drops" to a
timed double strike; it pairs naturally with an `my HP below` condition.

Example (a level 3 Knight with 4 nodes, plus the locked final rule):

```
1: [my HP below 25%?]   → [GUARD]
2: [ally HP below 50%?] → [SHIELD WALL]
3: [enemy adjacent?]    → [TAUNT]
4: [always]             → [PROTECT ALLY ▾ lowest HP]
🔒 attack nearest (advance if out of range)
```

## Prototype scope

Goal: prove the program → watch → analyse loop is fun, then add the shop.

- **Phase 1:** rule-card programs (done in the sim, all 6 classes), a small grid,
  hand-designed enemy boards, a drag-and-drop editor (`bevy_egui` is the likely quick
  path), and a replay showing which card each unit is running.
- **Phase 2:** the run loop: scenes, shop, bench, merging, economy, timed rounds,
  battle readability, post-game replay.
