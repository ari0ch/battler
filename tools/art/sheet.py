import os
from PIL import Image, ImageDraw
U = os.path.join(os.path.dirname(__file__), "out", "ui")
def load(n): return Image.open(os.path.join(U, n + ".png")).convert("RGBA")
def nine(img, s, w, h):
    """9-slice `img` (corner size s) to w x h at 1x."""
    W, H = img.size; out = Image.new("RGBA", (w, h))
    xs = [(0, s, 0, s), (s, W - s, s, w - s), (W - s, W, w - s, w)]
    ys = [(0, s, 0, s), (s, H - s, s, h - s), (H - s, H, h - s, h)]
    for (sx0, sx1, dx0, dx1) in xs:
        for (sy0, sy1, dy0, dy1) in ys:
            part = img.crop((sx0, sy0, sx1, sy1)).resize((dx1 - dx0, dy1 - dy0), Image.NEAREST)
            out.alpha_composite(part, (dx0, dy0))
    return out
S = 4
bg = (29, 26, 43, 255)
sheet = Image.new("RGBA", (1200, 760), bg)
x, y = 10, 10
def put(img, scale=S, gap=10):
    global x, y
    im = img.resize((img.width * scale, img.height * scale), Image.NEAREST)
    if x + im.width > 1190: x = 10; y += rowh + gap
    sheet.alpha_composite(im, (x, y)); x += im.width + gap
    return im
rowh = 0
def row(names, scale=S):
    global x, y, rowh
    rowh = max(load(n).height * scale for n in names)
    for n in names: put(load(n), scale)
    x = 10; y += rowh + 12
row(['panel_wood', 'panel_parchment', 'inset', 'button', 'button_hover', 'button_gold', 'button_pressed', 'card_row', 'wax_seal'])
row(['tag_condition', 'tag_action', 'tag_signature', 'tag_empty', 'bar_frame', 'fill_hp', 'fill_hp_low', 'fill_xp', 'fill_time', 'fill_enemy', 'banner'])
row(['frame_common', 'frame_rare', 'frame_epic', 'frame_enemy', 'cell_grass', 'cell_dirt'], 3)
row(['icon_' + n for n in 'coin heart star hourglass skull crown sword helm bow staff holy dagger axe'.split()], 5)
# stretched previews at 2x
rowh = 0
prev = [nine(load('panel_wood'), 10, 120, 70), nine(load('panel_parchment'), 10, 120, 70),
        nine(load('button'), 6, 70, 20), nine(load('button_gold'), 6, 70, 20),
        nine(load('tag_condition'), 4, 60, 12), nine(load('tag_action'), 4, 60, 12), nine(load('banner'), 6, 90, 14)]
rowh = max(p.height * 2 for p in prev)
for p in prev: put(p, 2)
sheet.save(os.path.join(os.path.dirname(__file__), "out", "_kit.png"))
