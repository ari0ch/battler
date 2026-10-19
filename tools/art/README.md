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
