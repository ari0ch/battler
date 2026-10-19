"""Battle scenes and animated previews for each perspective option."""
import os, math
from PIL import Image, ImageDraw
import perspective as P
from shapes import Canvas
from engine import PAL

OUT = os.path.join(os.path.dirname(__file__), "out", "persp")
ROSTER = os.path.join(os.path.dirname(__file__), "..", "..", "crates", "game", "assets", "sprites")
GRASS = Image.open(os.path.join(os.path.dirname(__file__), "out", "ui", "field_grass.png")).convert("RGBA")
def sprite(name):
    p = os.path.join(ROSTER, name + ".png")
    if not os.path.exists(p):
        p = os.path.join(ROSTER, "monsters", name + ".png")
    return Image.open(p).convert("RGBA")
def flip(im):
    return im.transpose(Image.FLIP_LEFT_RIGHT)
def up(im, s=2):
    return im.resize((im.width * s, im.height * s), Image.NEAREST)
def tile(w, h, src=GRASS):
    img = Image.new("RGBA", (w, h))
    for y in range(0, h, src.height):
        for x in range(0, w, src.width):
            img.alpha_composite(src, (x, y))
    return img
def shadow(img, cx, cy, rx, ry, rgba):
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    ImageDraw.Draw(layer).ellipse((cx - rx, cy - ry, cx + rx, cy + ry), fill=rgba)
    img.alpha_composite(layer)
def put(img, spr, cx, foot_y):
    img.alpha_composite(spr, (int(cx - spr.width / 2), int(foot_y - spr.height)))
BLUE, RED = (80, 150, 255, 110), (224, 72, 72, 110)
def frame_border(img):
    d = ImageDraw.Draw(img)
    d.rectangle((0, 0, img.width - 1, img.height - 1), outline=(26, 20, 38, 255))
    return img

# ---- walk frames for the 3/4 sprites: alternate the legs
def walk_frames(make, legs_y=25):
    base = make()
    frames = []
    for lift in (0, 1, 0, -1):
        c = make()
        if lift:
            # lift one leg by a pixel: move the left or right boot up
            g = c.g
            xs = range(10, 16) if lift > 0 else range(16, 22)
            for x in xs:
                for y in range(legs_y, 32):
                    g[y - 1][x] = g[y][x] if y < 31 else g[y - 1][x]
                g[31][x] = '.'
        frames.append(c.image())
    return frames

def gif(frames, path, scale=6, ms=160, bg=(52, 58, 72, 255)):
    out = []
    for f in frames:
        b = Image.new("RGBA", f.size, bg); b.alpha_composite(f)
        out.append(up(b, scale).convert("P", palette=Image.ADAPTIVE))
    out[0].save(path, save_all=True, append_images=out[1:], duration=ms, loop=0, disposal=2)

# ---- option 1: tilted field (foreshortened rows) with 3/4 sprites
def scene_three_quarter():
    cw, ch, lanes, rows = 32, 22, 7, 8
    W, H = cw * lanes, ch * rows
    img = tile(W, H)
    # depth cue: far rows a touch darker
    shade = Image.new("RGBA", (W, H), (0, 0, 0, 0)); d = ImageDraw.Draw(shade)
    for y in range(H):
        a = int(70 * (1 - y / H))
        d.line((0, y, W, y), fill=(20, 10, 30, a))
    img.alpha_composite(shade)
    d = ImageDraw.Draw(img)
    for x in range(0, W, 6):
        d.line((x, H // 2, x + 2, H // 2), fill=(243, 230, 196, 120))
    kb, kf, gf = P.knight_back().image(), P.knight_front().image(), P.goblin_front().image()
    for (lane, row, spr, col) in [(1, 1, gf, RED), (3, 1, gf, RED), (5, 2, gf, RED),
                                   (2, 5, kb, BLUE), (4, 5, kb, BLUE), (3, 7, kb, BLUE)]:
        cx, fy = lane * cw + cw / 2, row * ch + ch - 2
        shadow(img, cx, fy - 1, 9, 3, col)
        put(img, spr, cx, fy)
    return frame_border(up(img))

# ---- option 2: isometric diamonds
def scene_iso():
    tw, th, lanes, rows = 32, 16, 6, 6
    W, H = (lanes + rows) * tw // 2 + 4, (lanes + rows) * th // 2 + 40
    img = Image.new("RGBA", (W, H), (29, 26, 43, 255))
    t = Image.open(os.path.join(OUT, "iso_tile.png")).convert("RGBA")
    ox, oy = rows * tw // 2, 30
    for r in range(rows):
        for l in range(lanes):
            x = ox + (l - r) * tw // 2 - tw // 2
            y = oy + (l + r) * th // 2
            img.alpha_composite(t, (x, y))
    kb, gf = P.knight_back().image(), P.goblin_front().image()
    units = [(1, 1, gf, RED), (3, 0, gf, RED), (4, 2, gf, RED), (1, 4, kb, BLUE), (3, 4, kb, BLUE), (4, 5, kb, BLUE)]
    units.sort(key=lambda u: u[0] + u[1])
    for (l, r, spr, col) in units:
        cx = ox + (l - r) * tw // 2
        fy = oy + (l + r) * th // 2 + th // 2
        shadow(img, cx, fy, 9, 3, col)
        put(img, spr, cx, fy + 2)
    return frame_border(up(img))

# ---- option 3: true top-down
def scene_top_down():
    c, lanes, rows = 30, 7, 8
    W, H = c * lanes, c * rows
    img = tile(W, H)
    d = ImageDraw.Draw(img)
    for x in range(0, W, 6):
        d.line((x, H // 2, x + 2, H // 2), fill=(243, 230, 196, 120))
    kt, gt = P.knight_top().image(), P.goblin_top().image()
    gt = gt.transpose(Image.ROTATE_180)
    for (lane, row, spr, col) in [(1, 1, gt, RED), (3, 1, gt, RED), (5, 2, gt, RED),
                                   (2, 5, kt, BLUE), (4, 5, kt, BLUE), (3, 7, kt, BLUE)]:
        cx, cy = lane * c + c / 2, row * c + c / 2
        shadow(img, cx, cy + 2, 11, 9, col)
        img.alpha_composite(spr, (int(cx - 16), int(cy - 16)))
    return frame_border(up(img))

# ---- option 4: side-view battle (armies face each other left/right)
def scene_side():
    cw, ch, cols, rows = 30, 22, 8, 5
    W, H = cw * cols, ch * rows + 26
    img = tile(W, H)
    sky = Image.new("RGBA", (W, 26), (0, 0, 0, 0)); ImageDraw.Draw(sky).rectangle((0, 0, W, 26), fill=(150, 190, 230, 255))
    img.alpha_composite(sky, (0, 0))
    d = ImageDraw.Draw(img)
    d.rectangle((0, 24, W, 26), fill=(70, 110, 55, 255))
    for y in range(26, H, 6):
        d.line((W // 2, y, W // 2, y + 2), fill=(243, 230, 196, 120))
    left = [("bram_gatewarden", 3, 1), ("sir_aldric", 3, 3), ("pip_quickfeather", 1, 2), ("mirelle_frostweaver", 0, 4)]
    right = [("goblin_brute", 4, 1), ("goblin_brute", 4, 3), ("goblin_archer", 6, 2), ("ogre_chieftain", 5, 4)]
    for name, col, row in sorted(left + right, key=lambda u: u[2]):
        spr = sprite(name)
        enemy = (name, col, row) in right
        if enemy: spr = flip(spr)
        cx, fy = col * cw + cw / 2, 26 + row * ch + ch - 1
        shadow(img, cx, fy - 1, 9, 3, RED if enemy else BLUE)
        put(img, spr, cx, fy)
    return frame_border(up(img))

# ---- option 5: standees / board-game miniatures on round bases
def base(rim):
    c = Canvas(30, 12)
    c.ellipse(15, 6, 13, 4.5, rim)
    c.ellipse(15, 5, 12, 3.5, 'C', shade='B', hi='c')
    return c.outline().image()
def scene_standees():
    c, lanes, rows = 32, 7, 8
    W, H = c * lanes, c * rows
    img = tile(W, H)
    d = ImageDraw.Draw(img)
    for x in range(0, W, 6):
        d.line((x, H // 2, x + 2, H // 2), fill=(243, 230, 196, 120))
    units = [("goblin_brute", 1, 1, True), ("goblin_archer", 3, 0, True), ("goblin_brute", 5, 2, True),
             ("bram_gatewarden", 2, 5, False), ("kestrel", 4, 6, False), ("mirelle_frostweaver", 3, 7, False)]
    for name, lane, row, enemy in sorted(units, key=lambda u: u[2]):
        spr = sprite(name)
        if enemy: spr = flip(spr)
        cx, fy = lane * c + c / 2, row * c + c - 3
        b = base('R' if enemy else 'i')
        img.alpha_composite(b, (int(cx - 15), int(fy - 7)))
        put(img, spr, cx, fy - 2)
    return frame_border(up(img))

# ---- option 4 animation: profile sprite idle / walk / attack, made by bending the art
def side_frames(name="bram_gatewarden"):
    base_img = sprite(name)
    frames = []
    def shifted(dy=0, lean=0):
        f = Image.new("RGBA", base_img.size, (0, 0, 0, 0))
        for y in range(32):
            row = base_img.crop((0, y, 32, y + 1))
            off = int(round(lean * max(0, 24 - y) / 24))
            yy = y + (dy if y < 25 else 0)
            if 0 <= yy < 32:
                f.alpha_composite(row, (off, yy))
        return f
    # idle breathe, walk bob, wind-up, strike, recover
    for dy, lean in [(0, 0), (1, 0), (0, -2), (0, 3), (0, 1), (0, 0)]:
        frames.append(shifted(dy, lean))
    return frames

if __name__ == "__main__":
    scenes = {"scene_1_three_quarter": scene_three_quarter(), "scene_2_iso": scene_iso(),
              "scene_3_top_down": scene_top_down(), "scene_4_side": scene_side(),
              "scene_5_standees": scene_standees()}
    for n, im in scenes.items():
        im.save(os.path.join(OUT, n + ".png"))
    gif(walk_frames(P.knight_back), os.path.join(OUT, "anim_1_knight_back_walk.gif"))
    gif(walk_frames(P.goblin_front, legs_y=26), os.path.join(OUT, "anim_1_goblin_front_walk.gif"))
    gif(side_frames(), os.path.join(OUT, "anim_4_bram_attack.gif"), ms=140)
    gif(side_frames("goblin_brute"), os.path.join(OUT, "anim_4_goblin_attack.gif"), ms=140)
    print("done", {n: im.size for n, im in scenes.items()})
