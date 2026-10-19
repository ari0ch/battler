"""Render every character to out/<name>_idle.png plus review sheets."""
import os, sys
from PIL import Image, ImageDraw
from engine import render_grid, to_image, check
from characters import CHARACTERS, ORDER, OFFSET

OUT = os.path.join(os.path.dirname(__file__), "out")
os.makedirs(OUT, exist_ok=True)

imgs, problems = {}, False
for name in ORDER:
    try:
        g = render_grid(CHARACTERS[name], name, OFFSET.get(name, 0))
    except ValueError as e:
        print("ERROR", e); problems = True; continue
    w = check(g, name)
    if w:
        print(f"{name}: " + "; ".join(w))
    img = to_image(g)
    img.save(os.path.join(OUT, f"{name}_idle.png"))
    imgs[name] = img

# contact sheet: 6 columns (3 per class row pair) at chosen scale, classes as rows
def sheet(scale, path, bg=(52, 58, 72), label=False):
    cols, rows = 3, 6
    cell = 32 * scale + (8 if scale > 1 else 4)
    W, H = cols * 2 * cell // 2 * 2, rows * cell
    W = cols * cell
    s = Image.new("RGBA", (W * 2, H // 2 + cell // 2 if False else H), bg + (255,))
    s = Image.new("RGBA", (cols * 2 * cell, rows // 2 * cell), bg + (255,))
    for i, name in enumerate(ORDER):
        if name not in imgs: continue
        cls, k = divmod(i, 3)
        cx = (cls % 2) * 3 + k
        cy = cls // 2
        im = imgs[name].resize((32 * scale, 32 * scale), Image.NEAREST)
        s.alpha_composite(im, (cx * cell + 4, cy * cell + 4))
    s.save(path)

sheet(1, os.path.join(OUT, "_sheet_1x.png"))
sheet(2, os.path.join(OUT, "_sheet_2x.png"))
sheet(6, os.path.join(OUT, "_sheet_6x.png"))
sys.exit(1 if problems else 0)
