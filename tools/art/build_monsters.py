import os, sys
from PIL import Image
from grid import parse, image, bbox
from monsters import MONSTERS, OVERLAYS, SHIFT
OUT = os.path.join(os.path.dirname(__file__), "out"); os.makedirs(OUT, exist_ok=True)
imgs = {}
for name, text in MONSTERS.items():
    g = parse(text, name)
    for (y, x, span) in OVERLAYS.get(name, []):
        for i, ch in enumerate(span):
            if ch != '.':
                g[y][x + i] = ch
    dx = SHIFT.get(name, 0)
    if dx:
        g = [(['.'] * dx + row)[:32] if dx > 0 else (row[-dx:] + ['.'] * -dx) for row in g]
    x0, x1, y0, y1 = bbox(g)
    print(f"{name:16} x {x0:2}-{x1:2}  y {y0:2}-{y1:2}  centre {(x0+x1)/2:.1f}")
    imgs[name] = image(g)
    imgs[name].save(os.path.join(OUT, f"{name}.png"))
def sheet(scale, path):
    cell = 32 * scale + 8
    s = Image.new("RGBA", (4 * cell, 2 * cell), (52, 58, 72, 255))
    for i, (n, im) in enumerate(imgs.items()):
        s.alpha_composite(im.resize((32*scale, 32*scale), Image.NEAREST), ((i % 4) * cell + 4, (i // 4) * cell + 4))
    s.save(path)
sheet(6, os.path.join(OUT, "_monsters_6x.png")); sheet(2, os.path.join(OUT, "_monsters_2x.png"))
