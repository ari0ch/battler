# Art generators

Pixel art is drawn as code so it can be reviewed and changed in text. Needs Python 3
and Pillow. Output goes to `out/` next to the scripts; copy finished PNGs into
`crates/game/assets/`.

- `engine.py`, `characters.py`, `build.py`: the 18 roster characters (span ops on a
  32x32 grid, shared palette `PAL`).
- `grid.py`, `monsters.py`, `build_monsters.py`: the 8 PvE monsters, drawn as full
  32-row text grids in the same palette (one character per pixel, `.` is clear).
- `kit.py`: the fantasy UI kit (9-slice panels, buttons, card tags, rarity frames,
  bars, icons, ground). `sheet.py` renders a review sheet with 9-slice previews.

Everything is drawn at 1x and shown at whole-number scales (2x on the board).

## Perspective study

`shapes.py` (filled shapes on a small canvas with an automatic 1px outline),
`perspective.py` (example 3/4 front/back and top-down sprites, an isometric tile) and
`scenes.py` (a battle scene per option plus walk and attack GIFs) made the perspective
comparison in `docs/DESIGN.md`. Run `python3 perspective.py && python3 scenes.py`
after `kit.py` (scenes reuse `out/ui/field_grass.png`); output goes to `out/persp/`.

## Isometric

`iso.py` draws the back-facing (north-east) sprites and a facing sheet; `iso_scenes.py`
renders the diamond board and the isometric planning and battle mockups. Output goes to
`out/iso/`.
