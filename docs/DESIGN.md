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
- **Node count grows with unit level:** level 1 has 2 nodes, level 2 has 3, level 3 has 4
  (confirmed; synergies may add a slot on top).
- **Programming happens in the planning phase** between battles. Rounds are time-limited.
- **Blocks unlock as the player levels up.** A run starts with a small set of conditions and
  actions; each player level introduces a few more, with a notification saying what's new.
  This replaces "everything available from round 1", which playtesting found overwhelming.
- **Every block has a hover tooltip** explaining exactly what it does and when it falls
  through.
- **The block list has been reassessed** (see Block reassessment below). The reference
  tables further down describe the current build until it's implemented.

### Units and board

- **Units are unique, individually named characters**, each belonging to one class.
  The class sets the stat profile and counts towards class synergies; the **character**
  brings its own signature block(s) and, at higher rarities, a passive.
  (A role × origin grid was considered and rejected: named characters have more appeal,
  and the steeper learning curve is worth it.)
- **Rarity:** each class has characters at **3 rarities** (18 characters to start).
  Higher rarities cost more, appear later (via player level, below) and are stronger.
  Example: an early Common Knight brings Taunt; a later one brings Shield Wall or a self-heal.
- **Class synergies hook into programming** (see Synergies below): fielding several
  characters of a class unlocks things for your programs, not just stat bonuses.
- **One copy of each character on the board** (two different Knights are fine).
  Further copies sit inactive on the bench.
- **Bench: 9 slots** (as in TFT), enough to work on several upgrades at once while still
  forcing choices.
- **Levelling:** three copies merge into the next level (3 for level 2, 9 for level 3).
  The merged unit is the one on the board, so **its program carries over** and gains an
  empty node to fill. If no copy is on the board, the merged unit starts unprogrammed.

### Run structure

- **Eight-player lobby (Oct 2026).** You and seven bots. **Last one standing wins**;
  there is no round limit, and everyone gets a placement (1st to 8th).
- **Bots are players.** Each bot owns a full player state (HP, gold, level, XP, shop,
  bench, board) and changes it only by sending the same commands as the human (buy,
  sell, reroll, buy XP, move, set program). So every bot board is a real, legal board,
  and multiplayer can later replace bots with people, or let anyone view any board.
- **Shared pool.** Every shop draws from one pool: 29 copies of each Common, 22 of each
  Rare, 18 of each Epic. Knocked-out players' units return to it.
- **Round types.** Rounds 1-3 are monster rounds (Slimes, Wolf pack, Goblin raiders,
  each teaching one idea). A boss comes every 4 rounds from round 7 (Ogre Chieftain,
  Frost Wyrm, The Lich, then again, 80% stronger each cycle). Every other round is
  versus: random pairs, avoiding last round's opponent; with an odd number left, one
  player fights a **ghost** (a copy of another living player's board).
- **Player health:** 100 HP. A versus loss costs **round damage + the total level of the
  enemy units still alive**, where round damage is **2 + round + round²/6** (8 at round
  4, 20 at round 8, 38 at round 12, 54 at round 15). Gentle early, steep late; chosen in
  Oct 2026 to bring games down to about 20 minutes. A narrow loss still hurts less than
  a wipe, and a draw costs nothing. A monster round loss costs 2; a boss loss costs
  **round damage + 5**, always more than a versus loss in the same round. Losing to a
  ghost costs nothing.
- **Rewards:** +1 gold for a versus win, +2 for beating monsters, +4 for a boss.
  PvE rounds leave win/loss streaks alone.
- **Knock-outs:** players reaching 0 HP are out; if several fall in the same round,
  the one with the lowest (most negative) HP places lowest.
- **Battles spawn units alternating between the sides**, so neither side always acts
  first within a tick.
- **Player level** (TFT-style) replaces round-based board size. Level sets how many units
  may be on the board, the shop's rarity odds, and which blocks are unlocked.
  Starting numbers (to be checked by simulation later):
  - **Board size = player level.** Start at level 3; maximum level 8.
  - **XP:** +2 automatically per round; **buy 4 XP for 4 gold**.
    XP to reach the next level: 6, 10, 20, 36, 56 (levels 4 to 8).
  - **Shop odds** (Common / Rare / Epic):

    | Level | 3 | 4 | 5 | 6 | 7 | 8 |
    |---|---|---|---|---|---|---|
    | Odds % | 80/20/0 | 65/30/5 | 45/40/15 | 30/45/25 | 20/40/40 | 15/35/50 |
- **Battlefield (Oct 2026): 7 lanes wide, 4 rows deep a side** (was 8×5 with 5 lanes).
  Drawn with lanes across the screen and **the enemy half above the player's**. In sim
  coordinates `x` is depth (0 = the player's back row, 7 = the enemy's back row) and `y`
  is the lane; the game rotates this for display. Balance harness after the change: all
  classes 48-51%, lobbies still ~15 rounds.

### Scenes

**One round screen (Oct 2026, decluttering pass).** Planning, battle and the round result
all happen on one screen; there is no separate battle or result scene:
- The **battlefield is the centre of the screen** in both phases (full board, 66 px cells,
  your half with grid lines while planning, open grass in battle). An **up arrow** above it
  points at the enemy; hover it for who that is. No text around the battle.
- During planning the enemy half **previews the next opponent**: PvE waves stand where
  they will fight; versus rounds just say "Versus".
- **Synergies are icons only** (class icon, count, gold frame when active, faded when
  bench-only); details on hover. Bench is a 3x3 grid on the left; players on the right.
- The **program panel lies over the right side** and docks to a tab (Tab key or the tab
  button). Selecting a unit opens it; **pressing Ready docks it** for the battle.
- The **battle plays on the same field**; the header swaps the timer for speed/skip
  controls, and the shop shows "Battle!" instead of its buttons.
- The **round result is a small pop-up** over the next planning phase, whose **timer is
  already running** (needed for multiplayer, where rounds can't wait for one player). It
  closes itself after 6 seconds or on OK; the gold breakdown is in its tooltip.
- Replays from the run summary still have a screen of their own.

The notes below describe the earlier multi-scene flow and are kept for history.

Main menu → **run loop** (Planning → Battle → Result, repeated each round) → Game over →
Run summary → Main menu.

- **Main menu:** new run, how to play, settings.
- **Planning (timed): one screen, no modes.** Left to right: synergy tracker; a slim
  "next opponent" strip, **only the player's half of the board** (rotated, front row at
  the top) and the bench; the program column; and a **narrow standings sidebar** on the
  right (name, HP and a bar per player). The shop is the footer. (Oct 2026: the enemy
  half and the standings no longer share the board area.)
  The program column shows the selected unit's cards as single-line rows
  (`if [condition] -> [action]`) with the block palette **underneath**, not beside them.
  Clicking a unit makes its cards editable in place (no separate editor drawer; the
  drawer meant too many clicks and hid the shop). Hovering a signature block highlights
  its reach on the board.
- **Synergies are always visible.** Planning: a tracker on the left listing each class on
  the board with its count and thresholds; active synergies are highlighted, near-active
  ones show what one more unit would unlock, bench-only classes are faded. Shop offers
  say when buying would activate a synergy. Battle: a strip shows both sides' active
  synergies.
  60 seconds, or 75 once the player has 5+ units. A **Ready** button ends it early.
  The enemy board stays hidden until the battle starts.
- **Unit stats are a glance, not a panel:** one line in the program header, with full
  details in a hover tooltip on any unit, bench slot or shop offer.
- **Fewer clicks:** buying auto-places on the board when there's room; hotkeys D (reroll),
  E (sell hovered unit), F (buy XP); card-count dots on unit tiles show unprogrammed units.
- **Battle:** the full board, rotated (enemy above), with synergies and the legend in a
  left column. Watch the fight, at 1×/2×/4× speed or skipped to the end. On-board numbers
  and card badges are always on; the **card trace panel** is an optional toggle,
  **off by default** so new players aren't overwhelmed.
- **Result:** win/loss, HP lost, and a gold breakdown for next round.
- **Game over:** victory or defeat, rounds survived, then the run summary.
- **Run summary:** stats per round and a replay of any battle from its seed.
- **Art (Oct 2026):** 18 character sprites and 8 monster sprites, 32×32, facing right,
  drawn at 2× on the board and 1× on the bench, shop and program header; enemies are
  mirrored; **bosses are drawn at 3×** in battle. All embedded in the binary
  (`crates/game/assets/sprites`).
- **UI style (agreed Oct 2026): fantasy cartoon pixel art.** Inspired by Darkest Dungeon
  (heavy framed panels) and Ultima (parchment, wax seals), but bright and friendly, not
  gritty. Warm plum ground, wooden header and shop bars, gold-cornered wood panels for
  light text and parchment panels for dark ink text, plank and gold buttons, blue
  condition / orange action / purple signature tags, bronze / silver / gold rarity
  frames, kit bars and icons. Font: **Pixelify Sans** (OFL), embedded. The kit is drawn
  at 1x by `tools/art/kit.py` and shown at 2x. Mockups: the design canvas, row "Fantasy
  art pass".
- **Board look (agreed):** planning shows one grass field with grid lines over it (not
  separate tiles); battle shows open grass with no grid, a red tint on the enemy half
  and team-coloured shadows under units.
- **Battle movement (Oct 2026): free-looking, grid underneath.** The sim still moves
  units cell to cell and decides everything; the battle screen draws each unit as a
  free *body* (`crates/game/src/motion.rs`) that steers smoothly towards its cell at a
  small per-unit offset, pushes apart from overlapping bodies (collision radius about a
  third of the sprite, so bosses take more room), lunges at melee targets, is knocked
  back when hit, faces what it attacks or where it walks, and hops while walking.
  Ranged attacks fire a small projectile (orange for mages). Units lower on screen draw
  on top. Purely visual: replays and results are unchanged. Kept small so the picture
  stays honest about the grid rules (`adjacent`, `within 2 cells`).

### Shop and bench

All numbers in this section and the next are agreed starting points, to tune after playtesting.


- 5 offers per round. **Cost by rarity** (proposed 1 / 2 / 3 gold), **reroll 2 gold**,
  **sell** for the purchase price of the copies inside. Rarity odds depend on player level.
- **Bench: 9 slots.** Buying a third copy merges immediately, freeing slots.
- **Shared pool:** copies per character by rarity (proposed 12 / 10 / 8), drawn from the
  run's seeded RNG.
- **Programs persist between rounds.** Click any unit (board or bench) to edit its cards.

### Economy

- Start with **10 gold**.
- Each round: **5 base**, **+1 for a win**, **interest +1 per 10 gold banked** (max +5),
  **streak bonus** +1/+2/+3 at 2/4/6 consecutive wins *or* losses.

### Battle readability

The renderer turns each tick's events into short animations over the tick's duration.
In priority order:

1. **Card fired:** a badge above the acting unit showing the action. The optional card
   trace panel lists every unit's cards: the one that fired highlighted, the ones that
   fell through struck out.
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

## Playtest notes

- **Loss damage may be too low for 100 HP (Oct 2026, bot test).** A naive bot (buys
  anything, places by role, uses the AI's own programs) wins only ~3 of 15 battles yet
  reaches victory in 46% of runs, because ~12 losses at 2 + survivor levels don't add up to
  100 HP. Adding round-based damage, as TFT does with stage damage, makes losses matter:

  | Loss damage formula | Bot victories (200 runs) | Avg. final round |
  |---|---|---|
  | 2 + surviving enemy levels (current) | 46% | 14.7 |
  | 2 + round/2 + surviving enemy levels | 1.5% | 12.9 |
  | round + surviving enemy levels | 1% | 12.1 |

  Not changed yet: the current formula is the agreed starting point. Revisit after playing.
- **Balance harness, first pass (Oct 2026).** `cargo run -p battler-sim --example balance
  --release` (6000 random 3-6 a side fights, 300 bot runs). Before tuning, Rangers won 61%
  and Rogues 40%; after the class stat changes above, all classes sit at 48-51%. Remaining
  findings:
  - Within classes, Epics win 57-68% and Commons 39-49%, as intended, except **Vex (Rare,
    43%) is weaker than Nettle (Common, 47%)**: Opportunist only helps against wounded
    targets. Candidate for a buff.
  - **Seraphine (68%) and Ysolde (65%) are the strongest characters.**
  - **Synergies barely change random fights (49% win rate for the side with more active
    synergies)**, because preset programs don't use synergy blocks and random teams rarely
    reach tier 3. Needs a synergy-aware harness test before judging synergy strength.
  - **The naive bot reaches round ~12.5 and wins ~1 battle per run; 0% victories.** The AI
    (levels with the round, gets stars automatically) may be too strong for players who
    don't plan merges. Watch this in playtesting.
- **Turn order:** units now spawn alternating between the sides (Oct 2026).
- **Lobby harness (Oct 2026, 200 lobbies of 7 bots plus a naive player).** Bosses tuned
  so bots beat them about 70% of the time on their first appearance:

  | Round | Bot win rate |
  |---|---|
  | Slimes, Wolf pack, Goblin raiders (1-3) | 100% |
  | Ogre Chieftain (7) / Frost Wyrm (11) / The Lich (15) | 72% / 69% / 71% |
  | Second cycle (19 / 23 / 27) | 65% / 52% / 48% |

  Lobbies last **28.6 rounds on average** (23 to 35). At about 90 seconds a round, a
  winner plays ~40 minutes. A naive player (buys anything, no merge or synergy plan)
  places 7.5th on average: the bots are much stronger than someone who doesn't plan.
- **Game length cut to ~20 minutes (Oct 2026).** The harness now estimates screen time
  with every timer running out (planning 60-75 s, battle at 0.5 s a tick, 10 s result),
  an upper bound since players can press Ready. Before: winner ~46 min, average player
  ~36 min. Damage formulas tried (400 lobbies each; R = round, L = surviving enemy levels):

  | Versus loss damage | Avg. rounds | Winner min. | Avg. player min. |
  |---|---|---|---|
  | 2 + L (old) | 28.6 | 45.7 | 36.0 |
  | 2 + R + L | 20.6 | 32.5 | 25.7 |
  | 3R + L | 15.4 | 24.1 | 19.0 |
  | 2 + R + R²/8 + L | 16.5 | 25.7 | 20.7 |
  | **2 + R + R²/6 + L (chosen)** | **15.3** | **23.8** | **19.2** |

  The quadratic curve gets the same length as 3R while costing far less in the early
  versus rounds. Lengths flatten out around 15 rounds whatever the formula, because the
  last few players are evenly matched; shorter games from here would need shorter rounds.
  Side effect: games now end before boards fill out. **Bots average level 6.7 at round
  15**, so level 8 and Epic-heavy shops are rarely seen; the XP curve may need to speed
  up to match.

## Open questions

1. **More randomness?** Crits are implemented (see below). Dodges or damage ranges could
   follow if fights feel too predictable.
2. **Bot difficulty:** one bot strategy for now; difficulty levels may follow.
3. **Art direction and sourcing** for pixel art.
4. **Simulation pass** on player level numbers, shop odds and block unlocks once built.

## Roster (agreed draft, 18 characters)

Rarity rules: **Common** = one simple signature, no passive (teaches the class).
**Rare** = a stronger or more situational signature plus a passive. **Epic** = two
signatures, a passive, and slightly higher stats. Costs 1 / 2 / 3 gold; pool copies 12 / 10 / 8.

| Class | Common | Rare | Epic |
|---|---|---|---|
| Knight | **Bram Gatewarden**: Taunt | **Sir Aldric**: Shield Wall. *Bulwark*: -10% damage taken per adjacent ally | **Ysolde the Unbroken**: Taunt, Second Wind (heal self 30%). *Last Stand*: half damage below 25% HP |
| Ranger | **Pip Quickfeather**: Aimed Shot | **Kestrel**: Kite. *Hunter's Mark*: allies deal +15% to enemies she has hit | **Elowen Starshot**: Aimed Shot, Volley (half damage to every enemy in range). *Eagle Eye*: +1 range |
| Mage | **Tobble the Apprentice**: Firebolt (heavy single-target hit) | **Mirelle Frostweaver**: Frost Nova (adjacent enemies lose their next turn). *Chill*: frozen enemies take +20% | **Archmage Vorn**: Fireball, Blink. *Arcane Surge*: every third spell crits |
| Cleric | **Sister Agnes**: Heal | **Brother Calloway**: Sanctuary. *Blessing*: his heals also grant Guard | **Seraphine Dawnbringer**: Heal, Mass Heal (all allies). *Resurrection*: first ally to fall returns at 30% HP |
| Rogue | **Nettle**: Backstab | **Vex**: Shadowstep. *Opportunist*: +30% damage to enemies below half HP | **The Grey Widow**: Shadowstep, Assassinate (finish an enemy below 25% within 3 cells). *Vanish*: untargetable for the first 2 ticks |
| Berserker | **Grub Ironjaw**: Cleave | **Hilda Bloodaxe**: Frenzy. *Bloodlust*: heals 20% of damage dealt while below half HP | **Korgath the Undying**: Cleave, Rampage (Frenzy and Cleave at once). *Undying*: survives the first killing blow on 1 HP |

New sim mechanics this needs: freeze (skip a turn), mark, execute, untargetable,
resurrection, cheat-death, plus new abilities Firebolt, Frost Nova, Second Wind, Volley,
Mass Heal, Assassinate and Rampage. Numbers are first guesses for the balance harness.

## Synergies (agreed)

Thresholds **2 and 3** (three characters per class). At **2**, a new block unlocks; at
**3**, the class gets a stronger effect. Some unlocked blocks are **team-wide**
(coordination tools), others **class-only** (class tricks); the palette and tooltips label
which.

| Class | 2 units: new block | Scope | 3 units: class bonus |
|---|---|---|---|
| Knights | *ally attacked* (condition): an adjacent ally took damage last tick | Knights only | Knights get **+1 card slot** |
| Rangers | *marked* (target): the enemy a Ranger hit most recently | **Team** | Rangers get +1 range |
| Mages | *enemies grouped* (condition): 2+ enemies next to each other in range | Mages only | Mage signature cooldowns -2 |
| Clerics | **Send Signal** and **signal received** (red and blue) | **Team** | Receiving a signal heals 10% max HP |
| Rogues | *enemy isolated* (condition): an enemy with no allies within 2 cells | Rogues only | Rogue crit chance doubles (15% -> 30%) |
| Berserkers | *surrounded* (condition): 2+ enemies adjacent | Berserkers only | **Fury**: when a Berserker's card fires, the next card also runs if it can (one chain per tick) |

Rules:
- Synergies count **units on the board only**; each character counts once.
- If a synergy drops (a unit sold or benched), cards using its blocks are **kept but greyed
  out and skipped** in battle, and the tracker warns. Programs are never lost.
- **Bots build towards synergies** too (two focus classes each).
- All effects are first guesses for the balance harness.

## Block reassessment (agreed; to be checked by simulation)

- **Cut:** ability ready (fall-through already covers cooldowns), always (an empty
  condition means always), Advance (the locked rule advances), Hold, every N ticks.
- **Add:** *enemy weak* (an enemy in range below 30% HP), *I was hit* (took damage last tick).
- **Unlock order by player level:**

  | Level | Conditions | Actions | Targets |
  |---|---|---|---|
  | 3 (start) | enemy adjacent, my HP < X%, ally HP < X% | Attack, Guard | nearest, weakest |
  | 4 | enemy in range | Protect Ally | strongest |
  | 5 | outnumbered | Retreat | ranged |
  | 6 | enemy weak, I was hit | | |

  Signature blocks always come with their character. Signals become a synergy unlock.

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
- **The `it` target (Oct 2026, agreed after playtesting).** Conditions that spot a unit
  pass it to the action: `enemy adjacent`, `enemy in range`, `enemy weak` (the weakest
  such enemy), `was hit` (the attacker), `enemies grouped`, `enemy isolated` spot an
  enemy; `ally HP below` (the most hurt such ally) and `ally attacked` spot an ally. So
  `enemy weak -> Attack it` hits the weak enemy, not just the nearest, and `ally HP <
  50% -> Heal it` heals the ally that triggered it. Dropping an action after such a
  condition sets its target to `it` automatically; a program using `it` with a
  condition that can't supply a unit on the action's side is rejected. The target shows
  on the card as a pill, so it reads as a setting you can change by clicking.
- **Taunt overrides targeting:** a taunted unit targets its taunter with every
  enemy-targeted action, whatever its dropdown says.
- **Cooldowns:** each signature ability has its own cooldown. Abilities start ready.
- **Signals:** sent on one tick, visible to the whole team on the next tick only.
- **Defences:** Guard and Shield Wall halve damage (rounding up; they don't stack).
  Sanctuary blocks all damage.
- **Crits:** each hit has a 15% chance to deal 150% damage, from the battle's seeded RNG.
  Crit chance can be set to 0 per battle (useful for tests, or a no-luck mode).
- **Levels:** HP and attack scale ×1.8 per level (100% / 180% / 324%); range doesn't.
- **Freeze** (Frost Nova) costs the target its first turn *after* the tick it was frozen,
  so it is always exactly one turn regardless of turn order. Frozen units take +20%
  damage (Chill) until that turn.
- **Synergies in battle** are fixed when the battle starts, from each side's units. Cards
  needing an inactive synergy, or beyond a unit's card slots, are skipped.
- **Fury** (3 Berserkers): after a Berserker's card fires, the next card also runs if its
  condition holds and its action can run. Never chains from the locked default rule.
- **Resurrection** triggers once per battle, for the first ally to fall while Seraphine is
  alive. **Undying** triggers once. **Vanish** lasts ticks 1 and 2.

## Classes

| Class | HP | Attack | Range |
|---|---|---|---|
| Knight | 650 | 42 | 1 |
| Ranger | 300 | 44 | 3 |
| Mage | 350 | 45 | 3 |
| Cleric | 380 | 25 | 2 |
| Rogue | 480 | 72 | 1 |
| Berserker | 550 | 50 | 1 |

Level 1 stats of a Common. Rares get +10% HP and attack, Epics +20%. Signatures now come
from characters (see Roster). Tuned with the balance harness (Oct 2026) so every class
wins 48-51% of random team fights.

## Block reference (pre-phase 3; see Block reassessment and Synergies for the current set)

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
- **Phase 2 (first playable, in progress):** the run loop with all scenes, shop, bench,
  merging, economy, timed rounds, a card editor drawer, battle readability (badges,
  floating numbers, status letters, optional card trace) and post-game replays.
  Not yet: projectile/taunt lines, signal pulses, sound, art.
- **Phase 3 (built, Oct 2026):** one-screen planning layout, 18 named characters with
  rarities and passives, player level and XP, rarity shop odds, progressive block unlocks,
  hover tooltips, level-up notices, hotkeys, programming-hook synergies including Fury,
  synergy tracker and battle strip, and a headless balance harness.
- **Lobby (built, Oct 2026):** eight-player lobby with bots as players, shared pool,
  monster and boss rounds, ghosts, placements; the planning screen's enemy half previews
  the monster wave or shows the lobby standings.
- **Next candidates:** pixel art (32x32 sprites shown at 2x), projectile/taunt lines,
  synergy-aware bot programs, balance passes from playtesting.

## Upcoming development

In rough order. Each item is discussed before it is built.

1. **Done (Oct 2026): PvE rounds and the lobby.** See Run structure. Monsters are a
   separate roster (`crates/sim/src/monster.rs`) with fixed stats and programs; they never
   appear in the shop, pool, synergies or merges. Bosses use HP-phase programs.
2. **Playtest clean-up pass (Oct 2026 browser playtest):** header stats and the Buy XP /
   Reroll buttons wrap; long names overflow shop cards; the level-up notice covers the
   top of the board; the palette jumps down when a card row is added, causing misclicks;
   "1 ticks" on the result screen; a "what fired" per-unit summary on the result screen.
   **Done (Oct 2026):** dragging a unit onto an occupied cell swaps them (drops were being
   applied twice, undoing the swap); a block dropped on either half of a card replaces
   the matching half; **the palette shows only blocks the unit can use right now**
   (decided: locked blocks are hidden entirely, not greyed; level-up notices announce
   new ones).
3. **Items:** equipment dropped by PvE and boss rounds, to collect before the game
   proper starts (TFT-style). Not designed yet. Questions to settle: what items do (stats,
   new blocks, modified abilities?), how they combine, and how they are equipped in the
   one-screen planning layout.
4. **Viewing other boards:** click a player in the standings to see their board (the
   state is all there; it needs a screen).
5. **Done (Oct 2026): game length.** Round-scaled loss damage; ~15 rounds, ~20 minutes.
6. **XP curve for shorter games:** players rarely reach levels 7-8 before a lobby ends.
   Consider cheaper levels or more XP per round, checked with the harness.

## Roadmap (agreed Oct 2026)

Remaining phases in suggested order. Each is discussed before it is built; the
numbered list above stays as the record of what was done.

1. **Finish the decluttering pass (next).**
   - Check the one-screen layout live; fix spacing and overlapping text.
   - Proper art pass for the main menu and game over screens.
   - **"What fired" feedback:** the card a unit last ran on hover during battle, and a
     per-unit summary in the result pop-up ("Aimed Shot x3, Shield Wall never
     triggered"), so a loss can be debugged.
2. **Battle readability and feel.**
   - Death animations (fade or fall) instead of units vanishing.
   - Simple ability effects: Frost Nova ring, heal sparkles, taunt lines, signal pulses.
   - Tune movement feel; light screen shake on crits and boss hits.
3. **Items** (needs a design conversation first): equipment dropped by monster and boss
   rounds. To settle: what items change (stats, new blocks, modified abilities), whether
   they combine TFT-style, and where they live in the decluttered layout.
4. **Balance and opponents.**
   - Faster XP curve now games end around round 15 (levels 7-8 are rarely reached).
   - Bots that use the newer blocks (synergy blocks, the `it` target).
   - Bot difficulty levels (bots currently crush a player who doesn't plan merges).
   - Known outliers: Vex weaker than a Common; Seraphine and Ysolde strongest.
   - More monster and boss variety for long games.
5. **Programming quality of life.**
   - Copy a program between units; suggested starting programs per character.
   - "Why didn't this card fire?" explanations (no enemy in range, on cooldown...).
6. **Presentation.**
   - Animated sprites: 2-4 frame idle and attack loops from the same art scripts.
   - Sound and music (Bevy's audio feature; check the WASM size cost).
   - A short guided first run built on the three monster rounds; remembered settings
     (speed, volume).
7. **Multiplayer, in stages** (the lobby is deterministic, bots only act through player
   commands, and the result pop-up keeps the timer running, so the groundwork is in place).
   1. Asynchronous play: store each real player's board per round on a small backend and
      fight those recorded boards instead of bots.
   2. Accounts, saved runs and leaderboards.
   3. Live 8-player lobbies: a Rust server runs the `Lobby`; clients send commands;
      timer sync and reconnection.
   4. Viewing other players' boards from the standings (fits stage 1 or 3).
8. **Platform.**
   - Phone layout and touch controls.
   - A loading screen.
   - Screenshot-based UI tests in CI, to catch visual regressions.

