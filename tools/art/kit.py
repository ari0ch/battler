"""Fantasy-cartoon UI kit: small pixel-art PNGs drawn at 1x (shown at 2x).

Chunky dark outlines, warm wood and gold trim, parchment and coloured
gemstones; the roster palette plus a few UI shades. Frames are 9-slice:
corners stay crisp, edges and centres stretch or repeat.
"""
import os, random
from PIL import Image
from engine import PAL

OUT = os.path.join(os.path.dirname(__file__), "out", "ui"); os.makedirs(OUT, exist_ok=True)
C = {k: v + (255,) for k, v in PAL.items()}
C.update({
    'ink':   (26, 22, 38, 255),     # panel interior, deep plum
    'ink2':  (38, 32, 54, 255),     # raised interior
    'ink3':  (52, 44, 72, 255),
    'parch': (240, 226, 190, 255), 'parch2': (222, 202, 158, 255), 'parch3': (196, 170, 120, 255),
    'wood0': (54, 32, 22, 255), 'wood1': (92, 56, 34, 255), 'wood2': (128, 80, 46, 255), 'wood3': (166, 112, 64, 255),
    'grass0': (62, 104, 52, 255), 'grass1': (84, 132, 62, 255), 'grass2': (108, 158, 74, 255), 'grass3': (140, 184, 92, 255),
    'dirt0': (92, 62, 54, 255), 'dirt1': (118, 80, 66, 255), 'dirt2': (140, 98, 80, 255),
    'clear': (0, 0, 0, 0),
})
K = C['K']

def new(w, h, fill='clear'):
    return Image.new("RGBA", (w, h), C[fill] if isinstance(fill, str) else fill)

def px(img, x, y, c):
    if 0 <= x < img.width and 0 <= y < img.height:
        img.putpixel((x, y), C[c] if isinstance(c, str) else c)

def rect(img, x0, y0, x1, y1, c):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            px(img, x, y, c)

def rounded_box(img, x0, y0, x1, y1, outline='K', cut=1):
    """Outline with `cut` pixels clipped at each corner; returns the inner rect."""
    for x in range(x0 + cut, x1 - cut + 1):
        px(img, x, y0, outline); px(img, x, y1, outline)
    for y in range(y0 + cut, y1 - cut + 1):
        px(img, x0, y, outline); px(img, x1, y, outline)
    for i in range(1, cut + 1):  # diagonal corner steps
        px(img, x0 + i, y0 + cut - i + 1 - 1 + 0, outline) if False else None
    if cut:
        for (cx, cy, sx, sy) in ((x0, y0, 1, 1), (x1, y0, -1, 1), (x0, y1, 1, -1), (x1, y1, -1, -1)):
            for i in range(cut):
                px(img, cx + sx * (cut - i), cy + sy * (i + 1) - sy * 1 + sy * 0, outline) if False else None
            px(img, cx + sx * cut, cy + sy * 0, outline)
            px(img, cx + sx * 0, cy + sy * cut, outline)
            for i in range(1, cut):
                px(img, cx + sx * i, cy + sy * (cut - i), outline)
    return x0 + 1, y0 + 1, x1 - 1, y1 - 1

def fill_rows(img, x0, y0, x1, y1, shades, inside=None):
    """Vertical bands: shades[0] at the top ... shades[-1] at the bottom."""
    h = y1 - y0 + 1
    for y in range(y0, y1 + 1):
        c = shades[min(len(shades) - 1, (y - y0) * len(shades) // h)]
        for x in range(x0, x1 + 1):
            if inside is None or inside(x, y):
                px(img, x, y, c)

def stud(img, x, y):
    """A 3x3 gold rivet with a highlight."""
    for dx, dy, c in ((0,0,'Y'),(1,0,'G'),(2,0,'g'),(0,1,'G'),(1,1,'G'),(2,1,'g'),(0,2,'g'),(1,2,'g'),(2,2,'b')):
        px(img, x + dx, y + dy, c)

def save(img, name):
    img.save(os.path.join(OUT, name + ".png")); return img

rnd = random.Random(7)

# ---------------------------------------------------------------- panels
def bracket(img, x, y, sx, sy):
    """A 7x7 gold corner bracket (an L with a gem stud), mirrored by sx/sy."""
    shape = ["KKKKKKK",
             "KYGGGgK",
             "KGKKKKK",
             "KGK....",
             "KGK....",
             "KgK....",
             "KKK...."]
    for dy, row in enumerate(shape):
        for dx, ch in enumerate(row):
            if ch != '.':
                px(img, x + (dx if sx > 0 else -dx), y + (dy if sy > 0 else -dy), ch)

def wood_frame(inner='ink', name='panel_wood', size=32):
    """32x32, slice 10: thick plank border, bevelled, gold corner brackets."""
    img = new(size, size)
    s = size - 1
    rounded_box(img, 0, 0, s, s, cut=2)
    bands = [('wood3', 'wood1'), ('wood2', 'wood1'), ('wood2', 'wood0'), ('wood1', 'wood0')]
    for i, (lt, dk) in enumerate(bands):
        a, b = 1 + i, s - 1 - i
        for x in range(a + (1 if i == 0 else 0), b + 1):
            px(img, x, a, lt); px(img, x, b, dk)
        for y in range(a + (1 if i == 0 else 0), b + 1):
            px(img, a, y, lt); px(img, b, y, dk)
    r = random.Random(3)
    for _ in range(40):   # grain
        t = r.randrange(6, s - 6); d = r.randrange(2, 4)
        c = 'wood0' if r.random() < .6 else 'wood3'
        for (x, y) in ((t, d), (t, s - d), (d, t), (s - d, t)):
            if r.random() < .5: px(img, x, y, c)
    rect(img, 5, 5, s - 5, s - 5, 'K')                       # inner outline
    rect(img, 6, 6, s - 6, s - 6, inner)                     # plain centre: stretches cleanly
    hi = 'ink3' if inner == 'ink' else 'W'
    for x in range(6, s - 5): px(img, x, 6, hi)
    bracket(img, 1, 1, 1, 1); bracket(img, s - 1, 1, -1, 1)
    bracket(img, 1, s - 1, 1, -1); bracket(img, s - 1, s - 1, -1, -1)
    return save(img, name)

wood_frame()
wood_frame(inner='parch', name='panel_parchment')

# inset well (dark carved slot) for bench/board backing, 12x12 slice 4
def inset(name='inset', fill='ink'):
    img = new(12, 12)
    rounded_box(img, 0, 0, 11, 11, outline='wood0', cut=1)
    rect(img, 1, 1, 10, 10, fill)
    for x in range(2, 10): px(img, x, 1, 'V')
    for y in range(2, 10): px(img, 1, y, 'V')
    for x in range(2, 10): px(img, x, 10, 'ink3')
    return save(img, name)
inset()

# ---------------------------------------------------------------- buttons, 16x16 slice 6
def button(name, shades, edge_hi, edge_lo, gem=None):
    img = new(16, 16)
    rounded_box(img, 0, 0, 15, 15, cut=2)
    fill_rows(img, 1, 1, 14, 14, shades, inside=lambda x, y: not ((x in (1, 14)) and (y in (1, 14))))
    for x in range(3, 13): px(img, x, 2, edge_hi)
    for x in range(2, 14): px(img, x, 13, edge_lo)
    px(img, 2, 3, edge_hi)
    if gem:
        pass
    return save(img, name)
button('button', ['wood3', 'wood2', 'wood2', 'wood2', 'wood1', 'wood1'], 'c', 'wood0')
button('button_hover', ['c', 'wood3', 'wood3', 'wood3', 'wood2', 'wood2'], 'P', 'wood1')
button('button_gold', ['Y', 'G', 'G', 'G', 'G', 'g'], 'W', 'g')
button('button_pressed', ['wood1', 'wood1', 'wood2', 'wood2', 'wood2', 'wood2'], 'wood0', 'wood2')

# ---------------------------------------------------------------- card tags, 12x12 slice 4
def tag(name, shades, hi, lo, outline='K'):
    img = new(12, 12)
    rounded_box(img, 0, 0, 11, 11, outline=outline, cut=1)
    fill_rows(img, 1, 1, 10, 10, shades)
    for x in range(2, 10): px(img, x, 1, hi)
    for x in range(1, 11): px(img, x, 10, lo)
    return save(img, name)
tag('tag_condition', ['I', 'i', 'i', 'i'], 'J', 'S')
tag('tag_action', ['G', 'F', 'F', 'F'], 'Y', 'g')
tag('tag_signature', ['a', 'A', 'A', 'A'], 'J', 'v')
def tag_empty():
    img = new(12, 12)
    for x in range(1, 11):
        if x % 2: px(img, x, 0, 'parch3'); px(img, x, 11, 'parch3')
    for y in range(1, 11):
        if y % 2: px(img, 0, y, 'parch3'); px(img, 11, y, 'parch3')
    return save(img, 'tag_empty')
tag_empty()

# card row background: parchment strip with a wax-seal number circle drawn separately
def card_row():
    img = new(12, 12)
    rounded_box(img, 0, 0, 11, 11, outline='wood0', cut=1)
    fill_rows(img, 1, 1, 10, 10, ['parch', 'parch', 'parch2'])
    return save(img, 'card_row')
card_row()

def wax_seal():
    img = new(12, 12)
    pts = [(3,0),(4,0),(5,0),(6,0),(7,0),(8,0),(1,1),(2,1),(9,1),(10,1),(0,3),(11,3),(0,4),(0,5),(0,6),(0,7),(0,8),(11,4),(11,5),(11,6),(11,7),(11,8),(1,10),(2,10),(9,10),(10,10),(3,11),(4,11),(5,11),(6,11),(7,11),(8,11),(1,2),(10,2),(1,9),(10,9),(0,2),(11,2),(0,9),(11,9)]
    for y in range(12):
        for x in range(12):
            d = (x - 5.5) ** 2 + (y - 5.5) ** 2
            if d <= 30: px(img, x, y, 'R' if d > 6 else 'r')
            if 30 < d <= 38: px(img, x, y, 'K')
    for (x, y) in ((4, 3), (3, 4)): px(img, x, y, 'p')
    return save(img, 'wax_seal')
wax_seal()

# ---------------------------------------------------------------- unit frames, 36x36 slice 6
def unit_frame(name, metal, gem):
    m0, m1, m2, m3 = metal
    img = new(36, 36)
    rounded_box(img, 0, 0, 35, 35, cut=2)
    for i, c in enumerate((m3, m2)):
        a, b = 1 + i, 34 - i
        for x in range(a + 1, b): px(img, x, a, c)
        for y in range(a + 1, b): px(img, a, y, c)
    for i, c in enumerate((m0, m1)):
        a, b = 1 + i, 34 - i
        for x in range(a + 1, b + 1): px(img, x, b, c)
        for y in range(a + 1, b + 1): px(img, b, y, c)
    rounded_box(img, 3, 3, 32, 32, cut=0)
    # corner gems at the top
    if gem:
        g0, g1, g2 = gem
        for (x, y) in ((1, 1), (31, 1)):
            for dy, row in enumerate(("KKKK", "K21K", "K10K", "KKKK")):
                for dx, ch in enumerate(row):
                    px(img, x + dx, y + dy, {'K': 'K', '0': g0, '1': g1, '2': g2}[ch])
    return save(img, name)
unit_frame('frame_common', ('b', 'B', 'C', 'c'), None)
unit_frame('frame_rare', ('S', 'M', 'L', 'W'), ('i', 'I', 'J'))
unit_frame('frame_epic', ('g', 'G', 'Y', 'W'), ('v', 'a', 'J'))
unit_frame('frame_enemy', ('r', 'R', 'R', 'p'), None)

# ---------------------------------------------------------------- bars
def bar_frame():
    img = new(12, 8)
    rounded_box(img, 0, 0, 11, 7, cut=1)
    rect(img, 1, 1, 10, 6, 'V')
    for x in range(1, 11): px(img, x, 1, 'ink')
    return save(img, 'bar_frame')
bar_frame()
def bar_fill(name, hi, mid, lo):
    img = new(4, 6)
    for x in range(4):
        px(img, x, 0, hi); px(img, x, 1, mid); px(img, x, 2, mid); px(img, x, 3, mid); px(img, x, 4, lo); px(img, x, 5, lo)
    return save(img, name)
bar_fill('fill_hp', 'H', 'N', 'n')
bar_fill('fill_hp_low', 'R', 'R', 'r')
bar_fill('fill_xp', 'J', 'I', 'i')
bar_fill('fill_time', 'Y', 'G', 'g')
bar_fill('fill_enemy', 'p', 'R', 'r')

# ---------------------------------------------------------------- icons, 12x12
ICONS = {
 'coin': """
...KKKKK....
..KYYYGGK...
.KYWYGGGgK..
KYYGGGGGggK.
KYGGGgGGggK.
KYGGgGgGggK.
KGGGGgGGggK.
KGGGGGGGggK.
.KGGGGGggK..
..KgggggK...
...KKKKK....
............
""",
 'heart': """
.KKK...KKK..
KRWRK.KRRRK.
KRRRRKRRRrK.
KRRRRRRRRrK.
KRRRRRRRRrK.
.KRRRRRRrK..
..KRRRRrK...
...KRRrK....
....KrK.....
.....K......
............
............
""",
 'star': """
.....K......
....KYK.....
....KYK.....
KKKKYGYKKKK.
KYYYGGGGGgK.
.KKGGGGGgKK.
..KGGGGGgK..
..KGgKKGgK..
.KGgK..KgK..
.KgK....KgK.
.KK......KK.
............
""",
 'hourglass': """
.KKKKKKKKK..
.KccccccCK..
..KJJJJJK...
..KYYYYYK...
...KYYYK....
....KYK.....
...KJYJK....
..KJJYJJK...
..KYYYYYK...
.KccccccCK..
.KKKKKKKKK..
............
""",
 'skull': """
...KKKKK....
..KwwwwwK...
.KwwwwwwxK..
.KwKKwKKxK..
.KwKKwKKxK..
.KwwwKwwxK..
..KwwwwxK...
..KwKwKwK...
...KKKKK....
............
............
............
""",
 'crown': """
............
.K..K.K..K..
KGK KYK KGK.
KGKKGGGKKGK.
KGGGGGGGGGK.
KGRGGaGGRGK.
KGGGGGGGGgK.
KggggggggggK
.KKKKKKKKKK.
............
............
............
""",
 'sword': """
.........KK.
........KWLK
.......KWLK.
......KWLK..
.....KWLK...
.KK.KWLK....
.KGKKLK.....
..KGKK......
..KbKGK.....
.KbK.KGK....
KKK...KK....
............
""",
 'helm': """
...KKKKKK...
..KLLLLMSK..
.KLWLLLMMSK.
.KLLLLLMMSK.
.KMKKKKKKSK.
.KMLLVVVVSK.
.KMLLLLMMSK.
.KSMMMMMSSK.
..KKSSSSKK..
....KKKK....
............
............
""",
 'bow': """
..KK........
..KCK.......
...KCK......
...KCK.....K
....KCK...KM
....KCKKKMKK
...KKCKMMK..
....KCKKKMKK
....KCK...KM
...KCK.....K
..KCK.......
..KK........
""",
 'staff': """
.......KKK..
......KFYFK.
......KYWYK.
.......KFK..
.......KcK..
......KcK...
......KcK...
.....KcK....
.....KcK....
....KcK.....
....KKK.....
............
""",
 'holy': """
....KKK.....
....KYK.....
....KYK.....
.KKKKYKKKK..
.KYYYWYYYK..
.KKKKYKKKK..
....KYK.....
....KYK.....
....KgK.....
....KgK.....
....KKK.....
............
""",
 'dagger': """
..........K.
.........KLK
........KWK.
.......KWLK.
......KWLK..
.....KWLK...
..K.KLLK....
..KKMSK.....
...KAK......
..KAKK......
.KKK........
............
""",
 'arrow_up': """
.....KK.....
....KYYK....
...KYGGgK...
..KYGGGGgK..
.KYGGGGGGgK.
KKKKGGGGKKKK
...KGGGgK...
...KGGGgK...
...KGGGgK...
...KGggggK..
...KKKKKK...
............
""",
 'axe': """
....KKKK....
...KLLWLK...
..KLLLLMK.K.
..KMLLMKKcK.
...KMMKKcK..
....KKKcK...
.....KcK....
....KcK.....
...KcK......
..KbK.......
..KK........
............
""",
}
def icon(name, text):
    rows = [r for r in text.strip('\n').split('\n')]
    img = new(12, 12)
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            if ch not in '. ':
                px(img, x, y, ch)
    return save(img, 'icon_' + name)
for n, t in ICONS.items():
    icon(n, t)

# ---------------------------------------------------------------- ground
def ground_cell(name, base, light, dark, tuft, size=36, edge='K', seed=1):
    r = random.Random(seed)
    img = new(size, size)
    rounded_box(img, 0, 0, size - 1, size - 1, outline=dark, cut=2)
    rect(img, 1, 1, size - 2, size - 2, base)
    for x in range(2, size - 2): px(img, x, 1, light)
    for _ in range(size):
        x, y = r.randrange(3, size - 3), r.randrange(3, size - 3)
        px(img, x, y, light if r.random() < .5 else dark)
    for _ in range(4):
        x, y = r.randrange(4, size - 5), r.randrange(5, size - 4)
        px(img, x, y, tuft); px(img, x + 1, y - 1, tuft); px(img, x + 2, y, tuft)
    return save(img, name)
ground_cell('cell_grass', 'grass1', 'grass2', 'grass0', 'grass3', seed=3)
ground_cell('cell_dirt', 'dirt1', 'dirt2', 'dirt0', 'x', seed=5)

def field(name='field_grass', size=64, seed=11):
    """A seamless grass texture for the open battlefield (tiles with no seams)."""
    r = random.Random(seed)
    img = new(size, size, 'grass1')
    for _ in range(size * 3):
        x, y = r.randrange(size), r.randrange(size)
        px(img, x, y, 'grass2' if r.random() < .55 else 'grass0')
    for _ in range(14):
        x, y = r.randrange(size), r.randrange(size)
        for (dx, dy) in ((0, 0), (1, -1), (2, 0)):
            px(img, (x + dx) % size, (y + dy) % size, 'grass3')
    for _ in range(5):  # flowers
        x, y = r.randrange(size), r.randrange(size)
        c = r.choice(['Y', 'W', 'a'])
        for (dx, dy) in ((0, -1), (-1, 0), (1, 0), (0, 1)):
            px(img, (x + dx) % size, (y + dy) % size, c)
        px(img, x, y, 'G' if c != 'Y' else 'F')
    return save(img, name)
field()

# banner ribbon: 36x14, slice 10 left/right
def banner():
    img = new(40, 14)
    rect(img, 6, 1, 33, 11, 'R')
    for x in range(6, 34): px(img, x, 0, 'K'); px(img, x, 12, 'K'); px(img, x, 2, 'p'); px(img, x, 10, 'r'); px(img, x, 11, 'r')
    for y in range(0, 13): px(img, 5, y, 'K'); px(img, 34, y, 'K')
    # folded tails
    for y in range(3, 14):
        for x in range(0, 6):
            notch = abs(y - 8) < 3 - x // 2
            if not notch: px(img, x, y, 'r')
        for x in range(34, 40):
            notch = abs(y - 8) < 3 - (39 - x) // 2
            if not notch: px(img, x, y, 'r')
    for x in range(0, 6): px(img, x, 3, 'K'); px(img, 39 - x, 3, 'K'); px(img, x, 13, 'K'); px(img, 39 - x, 13, 'K')
    for y in range(3, 14): px(img, 0, y, 'K'); px(img, 39, y, 'K')
    return save(img, 'banner')
banner()

# header bar: wood planks, repeat-x, 32x30, gold trim at the bottom
def header():
    img = new(32, 30)
    fill_rows(img, 0, 0, 31, 25, ['wood2', 'wood1', 'wood1', 'wood1', 'wood1', 'wood0'])
    for y in (9, 18): 
        for x in range(32): px(img, x, y, 'wood0')
    for x in (10, 25): 
        for y in range(0, 9): px(img, x, y, 'wood0')
    for x in (4, 19): 
        for y in range(10, 18): px(img, x, y, 'wood0')
    for x in range(32): px(img, x, 26, 'g'); px(img, x, 27, 'G'); px(img, x, 28, 'g'); px(img, x, 29, 'K')
    for x in range(1, 32, 8): px(img, x, 27, 'Y')
    return save(img, 'header_wood')
header()
print(sorted(os.listdir(OUT)))
