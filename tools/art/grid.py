"""Render 32x32 text grids (one char per pixel, '.' = clear) with the roster palette."""
from PIL import Image
from engine import PAL

def parse(text, name):
    rows = [r for r in text.strip('\n').split('\n')]
    rows = [r.rstrip() for r in rows]
    if len(rows) > 32:
        raise ValueError(f"{name}: {len(rows)} rows")
    rows = ['.' * 32] * (32 - len(rows)) + rows   # bottom-align: feet on row 31
    g = []
    for y, r in enumerate(rows):
        if len(r) > 32:
            raise ValueError(f"{name}: row {y} is {len(r)} wide")
        r = r.ljust(32, '.')
        for x, ch in enumerate(r):
            if ch != '.' and ch not in PAL:
                raise ValueError(f"{name}: bad colour {ch!r} at ({x},{y})")
        g.append(list(r))
    return g

def image(g):
    img = Image.new("RGBA", (32, 32), (0, 0, 0, 0))
    for y, row in enumerate(g):
        for x, ch in enumerate(row):
            if ch != '.':
                img.putpixel((x, y), PAL[ch] + (255,))
    return img

def bbox(g):
    xs = [x for y in range(32) for x in range(32) if g[y][x] != '.']
    ys = [y for y in range(32) for x in range(32) if g[y][x] != '.']
    return min(xs), max(xs), min(ys), max(ys)
