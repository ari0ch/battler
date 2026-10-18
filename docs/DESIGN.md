# Battler: design notes

A living document. Decisions move from **Open questions** to **Decided** as we settle them.

## Pitch

A fantasy autobattler where you don't just pick and place units: you **program** them.
Each unit carries a tiny, Zachtronics-style program that governs its behaviour in battle.
The skill lies in planning, and in debugging your plan after it goes wrong.

## Pillars

1. **Small language, deep puzzles.** A handful of instructions and scarce program lines.
   Depth comes from combining them, not from a long instruction list.
2. **Readable failure.** You must always be able to see *why* a battle went wrong.
   The replay is a core feature, not polish.
3. **Deterministic battles.** The same armies and programs always produce the same fight.
   This makes debugging fair and keeps asynchronous PvP possible later.

## Decided

- **Engine and language:** Rust with Bevy (pinned at 0.20). This is also a Rust learning project.
- **Architecture:** a pure-Rust `battler-sim` crate with no engine dependencies holds all
  game rules and the interpreter. The Bevy `battler` crate only renders state, takes input,
  and hosts the editor and replay UI.
- **Primary target:** the web (WASM, WebGL2), deployed to GitHub Pages on every push to
  `main`, for easy playtest links. Desktop is used for the development loop only.

## Open questions

1. **When does programming happen?** Leaning towards *before* the battle (write, watch,
   debug, rerun), possibly with one live "interrupt" per battle.
2. **What does the editor look like?** Text-like lines (`IF ally.hp < 30% → TAUNT`), or
   chunky visual instruction tiles in the style of Opus Magnum?
3. **Long-term structure:** async PvP (Super Auto Pets style), a single-player roguelike
   run, or hand-designed puzzle encounters?

## Instruction language (sketch)

The current sim has a placeholder set (`Wait`, `Advance`, `Attack`, `Guard`), and programs
loop when they reach the end. Ideas for the real language:

- **Conditions:** priority-ordered `IF <condition> → <action>` rules, Gambit-style, mixed
  with plain sequential lines.
- **Signals between units:** `SEND <colour>` and `WAIT <colour>`, so units can coordinate
  across separate programs. This is likely the main source of puzzle depth.
- **One register per unit,** for counting, toggling, or remembering a target.
- **Scarce lines:** around 4–6 per unit. Extra lines and new instruction runes could be
  bought in the shop, which ties the economy to the programming layer.

Example:

```
1: IF ally.hp < 30% → MOVE TO ally; TAUNT
2: IF signal(RED)   → CHARGE nearest
3: WAIT signal(RED)
4: GUARD
```

## Prototype scope

Goal: prove the program → watch → debug loop is fun, before building any economy.

- A small grid, 3–4 unit types, about 8 instructions.
- Hand-designed enemy encounters (puzzle mode).
- A program editor (`bevy_egui` is the likely quick path).
- A replay with a scrubbable timeline, showing the line each unit is running.
- No shop or meta-progression yet.
