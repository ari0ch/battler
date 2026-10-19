"""Tiny pixel-art engine: characters are lists of drawing ops on a 32x32 grid.

Ops (strings):
  "y x STRING"          span; '.' in STRING is transparent
  "R y0 y1 x STRING"    repeat the span on every row y0..y1
  "L x0 y0 x1 y1 C"     1px line of colour C (Bresenham)
Ops (tuples):
  ("legs", kind, x, y, {"1": "L", ...})   stamp a leg template with colour map
"""
from PIL import Image

# Shared palette. Light comes from the top-left.
PAL = {
    'K': (26, 28, 44),    'V': (12, 12, 20),
    'd': (43, 43, 56),    'D': (71, 71, 89),    'e': (110, 110, 133), 'E': (157, 157, 176),
    'S': (74, 84, 104),   'M': (139, 155, 180), 'L': (192, 203, 220), 'W': (255, 255, 255),
    'b': (59, 36, 22),    'B': (93, 58, 36),    'C': (143, 90, 53),   'c': (196, 137, 90),
    'k': (160, 98, 74),   'p': (217, 154, 116), 'P': (245, 207, 168),
    'r': (110, 36, 54),   'R': (177, 62, 83),
    'F': (232, 118, 42),  'Y': (255, 210, 90),  'g': (168, 104, 28),  'G': (244, 180, 27),
    'n': (46, 74, 46),    'N': (79, 122, 60),   'H': (139, 176, 90),
    'o': (79, 107, 58),   'O': (122, 154, 80),  'q': (166, 192, 112),
    'i': (62, 122, 154),  'I': (124, 198, 220), 'J': (212, 242, 248),
    'v': (58, 42, 80),    'A': (99, 74, 135),   'a': (154, 122, 196),
    'w': (236, 230, 212), 'x': (189, 180, 156), 'X': (133, 124, 104),
}

LEGS = {
    # plate / trousers, 11 wide x 7 tall
    "std": [
        "K123K123K",
        "K123K123K",
        "K223K223K",
        "K333K333K",
        "K456K456K",
        "K4556K4556K",
        "KKKKKKKKKKK",
    ],
    # slim, 9 wide x 6 tall
    "slim": [
        "K12K12K",
        "K12K12K",
        "K33K33K",
        "K45K45K",
        "K455K455K",
        "KKKKKKKKK",
    ],
    # stout wide stance, 12 wide x 6 tall
    "stout": [
        "K123K..K123K",
        "K123K..K123K",
        "K223K..K223K",
        "K456K..K456K",
        "K4556KK4556K",
        "KKKKKKKKKKKK",
    ],
}


def _put(g, x, y, ch, name):
    if ch == '.':
        return
    if not (0 <= x < 32 and 0 <= y < 32):
        raise ValueError(f"{name}: pixel out of frame at ({x},{y})")
    if ch not in PAL:
        raise ValueError(f"{name}: unknown colour '{ch}' at ({x},{y})")
    g[y][x] = ch


def _span(g, y, x, s, name):
    for i, ch in enumerate(s):
        _put(g, x + i, y, ch, name)


def _line(g, x0, y0, x1, y1, ch, name):
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        _put(g, x0, y0, ch, name)
        if x0 == x1 and y0 == y1:
            break
        e2 = 2 * err
        if e2 >= dy:
            err += dy; x0 += sx
        if e2 <= dx:
            err += dx; y0 += sy


def render_grid(ops, name="?", dx=0):
    g = [['.'] * 32 for _ in range(32)]
    ops = shift(ops, dx)
    for op in ops:
        if isinstance(op, tuple) and op[0] == "legs":
            _, kind, x, y, cmap = op
            for dy, row in enumerate(LEGS[kind]):
                _span(g, y + dy, x, ''.join(cmap.get(c, c) for c in row), name)
            continue
        parts = op.split(' ')
        if parts[0] == 'L':
            x0, y0, x1, y1 = map(int, parts[1:5])
            _line(g, x0, y0, x1, y1, parts[5], name)
        elif parts[0] == 'R':
            y0, y1, x = map(int, parts[1:4])
            for y in range(y0, y1 + 1):
                _span(g, y, x, parts[4], name)
        else:
            _span(g, int(parts[0]), int(parts[1]), parts[2], name)
    return g


def to_image(g):
    img = Image.new("RGBA", (32, 32), (0, 0, 0, 0))
    for y, row in enumerate(g):
        for x, ch in enumerate(row):
            if ch != '.':
                img.putpixel((x, y), PAL[ch] + (255,))
    return img


def check(g, name):
    """Return a list of spec warnings: margins and footing."""
    xs = [x for y in range(32) for x in range(32) if g[y][x] != '.']
    ys = [y for y in range(32) for x in range(32) if g[y][x] != '.']
    warn = []
    if min(xs) < 2 or max(xs) > 29:
        warn.append(f"x-range {min(xs)}-{max(xs)}")
    if min(ys) < 2:
        warn.append(f"top row {min(ys)}")
    if max(ys) != 31:
        warn.append(f"feet at row {max(ys)} (want 31)")
    cx = (min(xs) + max(xs)) / 2
    if abs(cx - 15.5) > 2.5:
        warn.append(f"off-centre: bbox centre x={cx}")
    return warn


def shift(ops, dx):
    """Translate every op horizontally by dx pixels."""
    if not dx:
        return ops
    out = []
    for op in ops:
        if isinstance(op, tuple):
            out.append((op[0], op[1], op[2] + dx, op[3], op[4])); continue
        p = op.split(' ')
        if p[0] == 'L':
            p[1] = str(int(p[1]) + dx); p[3] = str(int(p[3]) + dx)
        elif p[0] == 'R':
            p[3] = str(int(p[3]) + dx)
        else:
            p[1] = str(int(p[1]) + dx)
        out.append(' '.join(p))
    return out
