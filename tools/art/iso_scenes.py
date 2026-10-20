"""Isometric battle and planning mockups (for the design canvas).

The board is the sim's 8 x 7 grid drawn as a diamond: depth (`x`) runs up and
to the right, lanes (`y`) down and to the right, so your half is bottom-left
and the enemy's top-right. Tiles are 48 x 24 art pixels, shown at 2x
(96 x 48), which makes the board 720 x 360 on the 1280 x 720 stage.
"""
import os, random, math
from PIL import Image, ImageDraw, ImageFont
from kit import C
import iso

HERE = os.path.dirname(__file__)
OUT = iso.OUT
FONT = os.path.join(HERE, "..", "..", "crates", "game", "assets", "fonts", "PixelifySans.ttf")

DEPTH, LANES = 8, 7
TW, TH = 48, 24          # tile, art pixels
THICK = 10               # the slab of earth under the board
S = 2                    # display scale
ORIGIN = (24, 96 + 60)   # cell (0, 0) centre; 60 px headroom for tall sprites
W, H = 360, 180 + 60 + THICK + 4


def centre(x, y):
    """Centre of cell (depth x, lane y), in art pixels."""
    return ORIGIN[0] + 24 * (x + y), ORIGIN[1] + 12 * (y - x)


def corner(i, j):
    """Grid corner i (0..DEPTH) along depth and j (0..LANES) along lanes."""
    return 24 * (i + j) + ORIGIN[0] - 24, ORIGIN[1] + 12 * (j - i)


def mix(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3)) + (255,)


def diamond(img, cx, cy, fill, edge=None, seed=0, tufts=True):
    """One tile. `fill(x, y)` gives each pixel's colour; edges get `edge`."""
    r = random.Random(seed)
    for row in range(TH):
        half = 2 * (row + 1) if row < TH // 2 else 2 * (TH - row)
        y = cy - TH // 2 + row
        for x in range(cx - half, cx + half):
            img.putpixel((x, y), fill(x, y))
        if edge and row < TH // 2:
            # the two upper edges are the grid lines; neighbours draw the rest
            img.putpixel((cx - half, y), edge); img.putpixel((cx - half + 1, y), edge)
            img.putpixel((cx + half - 1, y), edge); img.putpixel((cx + half - 2, y), edge)
    if tufts:
        for _ in range(3):
            x = cx + r.randint(-14, 12); y = cy + r.randint(-4, 4)
            for dx, dy in ((0, 0), (1, -1), (2, 0)):
                img.putpixel((x + dx, y + dy), C['grass3'])
        if r.random() < 0.3:
            x = cx + r.randint(-10, 10); y = cy + r.randint(-3, 3)
            col = C[r.choice('YWa')]
            for dx, dy in ((0, -1), (-1, 0), (1, 0), (0, 1)):
                img.putpixel((x + dx, y + dy), col)


def board(mode="battle", hover=None):
    """The diamond board at 1x. mode: "battle" (soft lines, warm enemy half)
    or "planning" (grid lines, the enemy half dimmed)."""
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    L = (centre(0, 0)[0] - 24, centre(0, 0)[1])
    B = (centre(0, LANES - 1)[0], centre(0, LANES - 1)[1] + 12)
    R = (centre(DEPTH - 1, LANES - 1)[0] + 24, centre(DEPTH - 1, LANES - 1)[1])
    # earth slab: left face lighter, right face darker, stones, a grass lip
    d.polygon([L, B, (B[0], B[1] + THICK), (L[0], L[1] + THICK)], fill=C['dirt1'])
    d.polygon([B, R, (R[0], R[1] + THICK), (B[0], B[1] + THICK)], fill=C['dirt0'])
    r = random.Random(4)
    for _ in range(40):
        t = r.random(); left = r.random() < 0.5
        a, b = (L, B) if left else (B, R)
        x = int(a[0] + (b[0] - a[0]) * t); y = int(a[1] + (b[1] - a[1]) * t) + r.randint(3, THICK - 2)
        img.putpixel((x, y), C['dirt2'] if left else C['dirt1'])
    d.line([L, B], fill=C['grass0'], width=3); d.line([B, R], fill=C['grass0'], width=3)
    d.line([(L[0], L[1] + THICK), (B[0], B[1] + THICK), (R[0], R[1] + THICK)], fill=C['K'], width=1)
    # tiles
    for x in range(DEPTH):
        for y in range(LANES):
            cx, cy = centre(x, y)
            enemy = x >= DEPTH // 2
            base, light = C['grass1'], C['grass2']
            if (x + y) % 2:
                base, light = mix(base, light, 0.35), mix(light, C['grass3'], 0.3)
            if enemy and mode == "battle":
                base, light = mix(base, (190, 120, 90), 0.10), mix(light, (200, 130, 100), 0.10)
            if enemy and mode == "planning":
                base, light = mix(base, (40, 40, 60), 0.45), mix(light, (40, 40, 60), 0.45)
            if hover == (x, y):
                base, light = mix(base, (255, 240, 170), 0.35), mix(light, (255, 240, 170), 0.35)
            rr = random.Random(x * 31 + y)
            fill = lambda px, py, base=base, light=light, rr=rr: light if rr.random() < 0.18 else base
            diamond(img, cx, cy, fill, None, seed=x * 7 + y * 13, tufts=not (enemy and mode == "planning"))
    if mode == "planning":
        # grid lines over all tiles, fainter on the enemy half
        mine = mix(C['grass0'], (20, 30, 20), 0.3)
        theirs = mix(C['grass0'], (30, 30, 45), 0.5)
        half = DEPTH // 2
        for i in range(DEPTH + 1):
            d.line([corner(i, 0), corner(i, LANES)], fill=mine if i <= half else theirs)
        for j in range(LANES + 1):
            d.line([corner(0, j), corner(half, j)], fill=mine)
            d.line([corner(half, j), corner(DEPTH, j)], fill=theirs)
    # outline the whole diamond so the board reads as one piece
    T = (centre(DEPTH - 1, 0)[0], centre(DEPTH - 1, 0)[1] - 12)
    d.line([L, T, R], fill=C['grass0'], width=1)
    # dashed midline between the halves
    for y in range(LANES):
        cx, cy = centre(DEPTH // 2 - 1, y)
        a = (cx, cy - 12); b = (cx + 24, cy)
        for i in range(0, 24, 4):
            p0 = (a[0] + i, a[1] + i // 2); p1 = (a[0] + i + 2, a[1] + (i + 2) // 2)
            d.line([p0, p1], fill=(236, 230, 212, 150) if mode == "planning" else (236, 230, 212, 90))
    return img


# ----------------------------------------------------------------- units
def shadow(img, cx, cy, team, scale=1.0):
    ring = (80, 150, 255, 150) if team == "you" else (225, 72, 72, 150)
    rx, ry = 12 * scale, 5 * scale
    ov = Image.new("RGBA", img.size, (0, 0, 0, 0))
    od = ImageDraw.Draw(ov)
    od.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=(20, 30, 20, 110), outline=ring, width=2)
    img.alpha_composite(ov)


def place(img, sprite, cx, cy, scale, team, alpha=1.0):
    """Stand a sprite with its feet on (cx, cy), all in display pixels."""
    shadow(img, cx, cy, team, scale / S)
    big = sprite.resize((32 * scale, 32 * scale), Image.NEAREST)
    if alpha < 1:
        a = big.getchannel("A").point(lambda v: int(v * alpha)); big.putalpha(a)
    img.alpha_composite(big, (int(cx - 16 * scale), int(cy - 30 * scale)))


def hp_bar(img, cx, top, pct, team, w=44):
    d = ImageDraw.Draw(img)
    x0 = cx - w // 2
    d.rectangle([x0 - 2, top - 2, x0 + w + 1, top + 7], fill=C['K'])
    d.rectangle([x0, top, x0 + w - 1, top + 5], fill=(60, 40, 40, 255))
    fill = (104, 196, 84, 255) if team == "you" else (214, 72, 72, 255)
    if team == "you" and pct < 0.35:
        fill = (232, 140, 50, 255)
    d.rectangle([x0, top, x0 + int((w - 1) * pct), top + 5], fill=fill)
    d.line([x0, top, x0 + int((w - 1) * pct), top], fill=(255, 255, 255, 90))


def text(img, xy, s, size, col, anchor="mm"):
    d = ImageDraw.Draw(img)
    f = ImageFont.truetype(FONT, size)
    d.text(xy, s, font=f, fill=col, anchor=anchor, stroke_width=2, stroke_fill=C['K'])


def badge(img, cx, top, label):
    d = ImageDraw.Draw(img)
    f = ImageFont.truetype(FONT, 12)
    w = d.textlength(label, font=f) + 16
    d.rounded_rectangle([cx - w / 2, top, cx + w / 2, top + 20], 4, fill=(99, 74, 135, 255),
                        outline=C['K'], width=2)
    d.text((cx, top + 10), label, font=f, fill=(244, 228, 255, 255), anchor="mm")


def ground_ring(img, cx, cy, r, col, width=4, alpha=200):
    """A ring on the ground: an ellipse twice as wide as tall (iso circles)."""
    ov = Image.new("RGBA", img.size, (0, 0, 0, 0))
    od = ImageDraw.Draw(ov)
    od.ellipse([cx - r, cy - r / 2, cx + r, cy + r / 2], outline=col + (alpha,), width=width)
    od.ellipse([cx - r * 0.8, cy - r * 0.4, cx + r * 0.8, cy + r * 0.4], fill=col + (50,))
    img.alpha_composite(ov)


def disp(x, y):
    cx, cy = centre(x, y)
    return cx * S, cy * S + 4


def unit_top(scale):
    return 30 * scale + 14


# ----------------------------------------------------------------- scenes
def battle():
    """Mid-fight against a PvE wave: a goblin warband led by the ogre chieftain."""
    img = board("battle").resize((W * S, H * S), Image.NEAREST)
    back = {n: iso.BACKS[n]().image() for n in iso.BACKS}
    flip = lambda im: im.transpose(Image.FLIP_LEFT_RIGHT)
    mon = lambda n: flip(iso.roster(n))
    # (cell, sprite, scale, team, hp, badge)
    units = [
        ((3, 2), back["bram_gatewarden"], 2, "you", 0.7, "Shield Wall"),
        ((3, 4), back["mirelle_frostweaver"], 2, "you", 0.9, "Frost Nova"),
        ((1, 1), back["elowen_starshot"], 2, "you", 1.0, None),
        ((1, 5), back["bram_gatewarden"], 2, "you", 0.3, None),
        ((4, 2), mon("goblin_brute"), 2, "foe", 0.4, None),
        ((4, 4), mon("wolf"), 2, "foe", 0.6, None),
        ((4, 5), mon("wolf"), 2, "foe", 0.8, None),
        ((6, 1), mon("goblin_archer"), 2, "foe", 1.0, None),
        ((6, 4), mon("ogre_chieftain"), 3, "foe", 0.85, None),
    ]
    # effects under the units: Frost Nova around Mirelle, Shield Wall around Bram
    mx, my = disp(3, 4)
    ground_ring(img, mx, my, 110, (158, 224, 255), 5)
    bx, by = disp(3, 2)
    ground_ring(img, bx, by, 60, (115, 178, 255), 3, 170)
    for (x, y), sp, sc, team, hp, label in sorted(units, key=lambda u: disp(*u[0])[1]):
        cx, cy = disp(x, y)
        place(img, sp, cx, cy, sc, team)
        top = cy - unit_top(sc)
        hp_bar(img, cx, top, hp, team, 44 if sc == 2 else 70)
        if label:
            badge(img, cx, top - 26, label)
    # ice shards flying out of the nova
    d = ImageDraw.Draw(img)
    for i in range(14):
        a = i / 14 * math.tau
        r = 70 + (i % 3) * 14
        x, y = mx + math.cos(a) * r, my + math.sin(a) * r / 2 - 10
        d.rectangle([x - 3, y - 3, x + 3, y + 3], fill=(212, 242, 248, 230))
    # Elowen's arrow on its way to the goblin archer
    ex, ey = disp(1, 1); gx, gy = disp(6, 1)
    t = 0.55
    ax, ay = ex + (gx - ex) * t, ey - 40 + (gy - ey) * t - 30
    d.line([ax - 14, ay + 6, ax + 6, ay - 2], fill=C['c'], width=3)
    d.polygon([(ax + 6, ay - 6), (ax + 12, ay - 4), (ax + 6, ay + 2)], fill=C['L'])
    # hits: the wolf frozen, the brute knocked back
    wx, wy = disp(4, 4)
    text(img, (wx + 18, wy - 92), "-14", 20, (255, 120, 110, 255))
    text(img, (wx - 26, wy - 74), "Frozen", 14, (158, 224, 255, 255))
    gx2, gy2 = disp(4, 2)
    text(img, (gx2 + 16, gy2 - 96), "31!", 26, (255, 210, 90, 255))
    for i in range(8):
        a = i / 8 * math.tau
        d.rectangle([gx2 + math.cos(a) * 26 - 2, gy2 - 40 + math.sin(a) * 18 - 2,
                     gx2 + math.cos(a) * 26 + 2, gy2 - 40 + math.sin(a) * 18 + 2], fill=(255, 210, 90, 255))
    return img


def planning():
    """Planning: grid lines on your half, the next wave shown faintly on theirs,
    your units facing the camera so you can see who is who."""
    img = board("planning", hover=(0, 3)).resize((W * S, H * S), Image.NEAREST)
    flip = lambda im: im.transpose(Image.FLIP_LEFT_RIGHT)
    units = [
        ((3, 2), iso.roster("bram_gatewarden"), 2, "you", 1.0),
        ((3, 4), iso.roster("mirelle_frostweaver"), 2, "you", 1.0),
        ((1, 1), iso.roster("elowen_starshot"), 2, "you", 1.0),
        ((1, 5), iso.roster("sister_agnes"), 2, "you", 1.0),
        ((4, 2), flip(iso.roster("goblin_brute")), 2, "foe", 0.45),
        ((4, 4), flip(iso.roster("wolf")), 2, "foe", 0.45),
        ((4, 5), flip(iso.roster("wolf")), 2, "foe", 0.45),
        ((6, 1), flip(iso.roster("goblin_archer")), 2, "foe", 0.45),
        ((6, 4), flip(iso.roster("ogre_chieftain")), 3, "foe", 0.45),
    ]
    for (x, y), sp, sc, team, alpha in sorted(units, key=lambda u: disp(*u[0])[1]):
        cx, cy = disp(x, y)
        place(img, sp, cx, cy, sc, team, alpha)
    # a unit being dragged onto the highlighted cell
    hx, hy = disp(0, 3)
    ghost = iso.roster("hilda_bloodaxe")
    place(img, ghost, hx, hy - 10, 2, "you", 0.85)
    return img


def tile_overlay(fill, edge):
    """A one-tile diamond overlay (hover, reach) at 1x."""
    img = Image.new("RGBA", (TW, TH), (0, 0, 0, 0))
    for row in range(TH):
        half = 2 * (row + 1) if row < TH // 2 else 2 * (TH - row)
        for x in range(TW // 2 - half, TW // 2 + half):
            img.putpixel((x, row), fill)
        for x in (TW // 2 - half, TW // 2 - half + 1, TW // 2 + half - 2, TW // 2 + half - 1):
            img.putpixel((x, row), edge)
    return img


def export(dest):
    """The game's board pieces: both boards cropped to the diamond and its slab
    (360 x 192 at 1x; the top corner is at the top edge) and the overlays."""
    top = ORIGIN[1] - 12 * DEPTH
    box = (0, top, W, top + 180 + THICK + 2)
    board("battle").crop(box).save(os.path.join(dest, "board_battle.png"))
    board("planning").crop(box).save(os.path.join(dest, "board_planning.png"))
    tile_overlay((255, 240, 170, 80), (255, 240, 170, 200)).save(os.path.join(dest, "tile_hover.png"))
    tile_overlay((120, 190, 255, 80), (150, 210, 255, 200)).save(os.path.join(dest, "tile_reach.png"))


if __name__ == "__main__":
    export(OUT)
    battle().save(os.path.join(OUT, "iso_battle.png"))
    planning().save(os.path.join(OUT, "iso_planning.png"))
    print("ok")
