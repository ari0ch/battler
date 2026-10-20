"""Item icons: 16x16 pixel art (shown at 2x), drawn with shapes and an
automatic outline in the roster palette. Run `python3 items.py`; output goes
to out/items/, then copy into crates/game/assets/ui/items/."""
import os
from PIL import Image
from shapes import Canvas

OUT = os.path.join(os.path.dirname(__file__), "out", "items"); os.makedirs(OUT, exist_ok=True)


def c16():
    return Canvas(16, 16)


def iron_sword():
    c = c16()
    c.line(4, 11, 12, 3, 'L'); c.line(5, 11, 13, 3, 'M'); c.px(13, 2, 'W'); c.px(12, 2, 'L')
    c.line(2, 10, 6, 14, 'G'); c.px(4, 12, 'g')          # guard
    c.line(2, 14, 3, 13, 'B'); c.px(1, 15, 'C')          # grip and pommel
    return c.outline()


def leather_jerkin():
    c = c16()
    c.poly([(3, 3), (6, 2), (10, 2), (13, 3), (13, 14), (3, 14)], 'C')
    c.poly([(9, 2), (10, 2), (13, 3), (13, 14), (9, 14)], 'B')
    c.rect(1, 3, 3, 8, 'C'); c.rect(13, 3, 15, 8, 'B')    # sleeves
    c.poly([(6, 2), (10, 2), (8, 6)], '.')                # neck
    c.line(8, 6, 8, 13, 'b'); c.px(7, 8, 'G'); c.px(7, 11, 'G')
    return c.outline()


def lucky_coin():
    c = c16()
    c.ellipse(8, 8, 6, 6, 'G', shade='g', hi='Y')
    c.ellipse(8, 8, 4, 4, 'Y')
    # a four-leaf clover
    for x, y in ((7, 6), (9, 6), (7, 8), (9, 8)):
        c.rect(x - 1, y - 1, x, y, 'N')
    c.px(8, 10, 'n'); c.px(9, 11, 'n')
    return c.outline()


def healing_draught():
    c = c16()
    c.ellipse(8, 10, 5, 4.5, 'R', shade='r', hi='P')
    c.rect(6, 3, 10, 6, 'J'); c.rect(6, 2, 10, 2, 'C')   # neck, cork
    c.rect(7, 1, 9, 1, 'B')
    c.px(6, 8, 'W'); c.px(6, 9, 'W')
    return c.outline()


def whetstone():
    c = c16()
    c.poly([(2, 10), (11, 4), (14, 7), (5, 13)], 'E')
    c.poly([(5, 13), (14, 7), (14, 9), (5, 15)], 'e')
    c.line(4, 10, 11, 5, 'L')
    c.px(7, 9, 'W'); c.px(10, 7, 'W')
    return c.outline()


def scouts_map():
    c = c16()
    c.rect(2, 3, 13, 12, 'x'); c.rect(2, 3, 13, 4, 'w')
    c.rect(1, 2, 2, 13, 'c'); c.rect(13, 2, 14, 13, 'c')  # rolled ends
    c.line(4, 10, 7, 7, 'X'); c.line(7, 7, 9, 8, 'X')       # dotted path
    c.px(10, 6, 'R'); c.px(11, 5, 'R'); c.px(11, 7, 'R'); c.px(12, 6, 'R'); c.px(11, 6, 'R')
    return c.outline()


def spyglass():
    c = c16()
    c.line(2, 12, 11, 3, 'G'); c.line(3, 13, 12, 4, 'g')
    c.line(2, 13, 9, 6, 'g')
    c.ellipse(12.5, 3, 2, 2, 'C', shade='B'); c.px(12, 3, 'J')
    c.line(5, 10, 6, 11, 'B'); c.line(8, 7, 9, 8, 'B')     # bands
    return c.outline()


def vampire_fang():
    c = c16()
    c.poly([(4, 2), (12, 2), (9, 13), (8, 14), (7, 13)], 'w')
    c.poly([(9, 2), (12, 2), (9, 13)], 'x')
    c.rect(4, 2, 12, 3, 'P')                              # gum line
    c.px(9, 14, 'R'); c.rect(9, 15, 10, 15, 'R'); c.px(10, 13, 'r')
    return c.outline()


def spiked_buckler():
    c = c16()
    c.ellipse(8, 8, 5, 5, 'C', shade='B', hi='c')
    c.ellipse(8, 8, 2, 2, 'M', shade='S', hi='L')
    for x, y in ((8, 1), (8, 15), (1, 8), (15, 8), (3, 3), (13, 3), (3, 13), (13, 13)):
        c.px(x, y, 'L')
    return c.outline()


def hourglass():
    c = c16()
    c.rect(3, 1, 13, 2, 'B'); c.rect(3, 14, 13, 15, 'B')
    c.poly([(4, 3), (12, 3), (8, 8)], 'J'); c.poly([(8, 8), (12, 13), (4, 13)], 'J')
    c.poly([(6, 5), (10, 5), (8, 8)], 'Y'); c.poly([(8, 10), (11, 13), (5, 13)], 'Y')
    c.px(8, 9, 'Y')
    c.line(3, 3, 3, 13, 'C'); c.line(13, 3, 13, 13, 'C')
    return c.outline()


def phoenix_feather():
    c = c16()
    c.poly([(3, 14), (6, 7), (12, 1), (13, 3), (9, 10)], 'F')
    c.poly([(6, 9), (12, 2), (13, 3), (9, 10)], 'R')
    c.line(3, 14, 12, 2, 'Y')                              # quill
    c.px(5, 9, 'Y'); c.px(8, 5, 'Y'); c.px(10, 8, 'r')
    return c.outline()


# Each class's banner colour: (main, shade).
BANNER = {
    "knight": ('M', 'S'), "ranger": ('N', 'n'), "mage": ('A', 'v'),
    "cleric": ('w', 'x'), "rogue": ('D', 'd'), "berserker": ('R', 'r'),
}


def banner(main, shade):
    c = c16()
    c.line(3, 1, 3, 15, 'B'); c.px(3, 0, 'G')
    c.poly([(4, 2), (13, 2), (13, 10), (8, 8), (4, 10)], main)
    c.poly([(10, 2), (13, 2), (13, 10), (10, 9)], shade)
    c.rect(4, 2, 13, 2, 'G')
    c.px(8, 5, 'G'); c.px(7, 5, 'Y'); c.px(9, 5, 'Y'); c.px(8, 4, 'Y'); c.px(8, 6, 'Y')
    return c.outline()


def twin_blades():
    c = c16()
    c.line(2, 2, 11, 11, 'L'); c.line(3, 2, 12, 11, 'M')
    c.line(13, 2, 4, 11, 'L'); c.line(14, 2, 5, 11, 'M')
    c.line(10, 12, 13, 9, 'G'); c.line(2, 9, 5, 12, 'G')
    c.line(12, 12, 14, 14, 'B'); c.line(3, 12, 1, 14, 'B')
    return c.outline()


def aegis():
    c = c16()
    c.poly([(2, 2), (14, 2), (14, 8), (8, 15), (2, 8)], 'G')
    c.poly([(8, 2), (14, 2), (14, 8), (8, 15)], 'g')
    c.poly([(4, 4), (12, 4), (12, 8), (8, 12), (4, 8)], 'J')
    c.poly([(8, 4), (12, 4), (12, 8), (8, 12)], 'I')
    c.px(8, 6, 'W'); c.px(8, 7, 'W'); c.px(7, 7, 'W'); c.px(9, 7, 'W'); c.px(8, 8, 'W')
    return c.outline()


ICONS = {
    "iron_sword": iron_sword, "leather_jerkin": leather_jerkin, "lucky_coin": lucky_coin,
    "healing_draught": healing_draught, "whetstone": whetstone, "scouts_map": scouts_map,
    "spyglass": spyglass, "vampire_fang": vampire_fang, "spiked_buckler": spiked_buckler,
    "hourglass": hourglass, "phoenix_feather": phoenix_feather, "twin_blades": twin_blades,
    "aegis": aegis,
}
for cls, (m, s) in BANNER.items():
    ICONS[f"banner_{cls}"] = (lambda m=m, s=s: banner(m, s))

if __name__ == "__main__":
    for name, f in ICONS.items():
        f().image().save(os.path.join(OUT, name + ".png"))
    # review sheet at 4x
    names = list(ICONS)
    sheet = Image.new("RGBA", (len(names) * 72, 72), (240, 226, 190, 255))
    for i, n in enumerate(names):
        im = Image.open(os.path.join(OUT, n + ".png")).resize((64, 64), Image.NEAREST)
        sheet.paste(im, (i * 72 + 4, 4), im)
    sheet.save(os.path.join(OUT, "_sheet.png"))
    print(len(names), "icons")
