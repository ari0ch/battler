"""Isometric look: back-facing roster sprites and the diamond battlefield.

The roster sprites already face right in a front-3/4 view, so on the diamond
they serve as the "towards the camera" facings (south-east as drawn, mirrored
for south-west). This file adds the "away from the camera" facings: the same
characters seen from behind, walking north-east (mirrored for north-west).

Run `python3 iso.py` to write `out/iso/`: the back sprites, a facing sheet and
the board pieces.
"""
import os
from PIL import Image
from shapes import Canvas

HERE = os.path.dirname(__file__)
OUT = os.path.join(HERE, "out", "iso"); os.makedirs(OUT, exist_ok=True)
ROSTER = os.path.join(HERE, "..", "..", "crates", "game", "assets", "sprites")


# ----------------------------------------------------------------- back views
# Proportions follow the roster: helm or hair from row 4, body rows 15-24,
# legs to row 30, feet on row 30/31. Facing north-east means we see the back
# and a sliver of the right side; the right hand is on screen right.

def bram_back():
    """Bram Gatewarden from behind: kettle helm, mail coif, tabard, shield on
    his left arm (screen left, its wooden back towards us), sword raised."""
    c = Canvas()
    # legs and boots, right leg a step ahead (higher)
    c.rect(12, 25, 14, 28, 'M'); c.rect(14, 25, 14, 28, 'S')
    c.rect(16, 24, 18, 27, 'M'); c.rect(18, 24, 18, 27, 'S')
    c.rect(12, 29, 14, 30, 'B'); c.rect(12, 30, 15, 30, 'b')
    c.rect(16, 28, 18, 29, 'B'); c.rect(16, 29, 19, 29, 'b')
    # tabard back over mail
    c.rect(11, 15, 19, 24, 'B'); c.rect(18, 15, 19, 24, 'b'); c.rect(11, 16, 11, 23, 'C')
    c.rect(11, 21, 19, 21, 'b')                       # belt
    c.rect(15, 16, 15, 20, 'b'); c.rect(15, 22, 15, 24, 'b')  # back seam
    c.rect(10, 15, 20, 16, 'M'); c.rect(18, 15, 20, 16, 'S')  # mail shoulders
    # sword arm, reaching forward and up to the right
    c.rect(20, 16, 21, 19, 'M'); c.rect(21, 17, 21, 19, 'S')
    c.px(22, 17, 'p'); c.px(22, 18, 'k')
    c.line(23, 15, 28, 5, 'L'); c.line(24, 15, 29, 6, 'M'); c.px(28, 4, 'W')
    c.line(21, 14, 25, 17, 'G'); c.px(23, 16, 'g')
    # shield on the left arm: wooden back, an iron rim and a leather strap
    c.ellipse(7, 19, 4, 5.5, 'C', shade='B', hi='c')
    c.line(6, 14, 6, 24, 'B'); c.line(8, 14, 8, 24, 'B')
    c.rect(4, 18, 10, 19, 'b')
    c.rect(9, 16, 10, 20, 'M')
    # kettle helm from behind: a low dome on a wide brim
    c.ellipse(15, 7, 4.5, 2.5, 'L', shade='M', hi='W')
    c.rect(9, 9, 21, 9, 'S'); c.rect(10, 8, 20, 8, 'M')
    # mail coif down to the shoulders, a little cheek on the right
    c.rect(12, 10, 18, 13, 'M'); c.rect(17, 10, 18, 13, 'S'); c.rect(12, 10, 12, 12, 'L')
    c.rect(13, 14, 17, 14, 'S')
    c.px(19, 11, 'p'); c.px(19, 12, 'k')
    return c.outline()


def mirelle_back():
    """Mirelle Frostweaver from behind: pointed ice-blue hood and mantle over
    a long robe with a snowflake on the back, frost staff in her right hand."""
    c = Canvas()
    # robe widening to the hem
    c.poly([(10, 16), (20, 16), (22, 30), (8, 30)], 'I')
    c.poly([(16, 16), (20, 16), (22, 30), (16, 30)], 'i')
    c.poly([(10, 16), (12, 16), (11, 30), (8, 30)], 'J')
    c.rect(8, 30, 21, 30, 'w')                          # hem trim
    c.line(15, 23, 15, 29, 'i')                         # back pleat
    # snowflake on the back
    c.px(15, 19, 'W'); c.px(14, 20, 'W'); c.px(16, 20, 'W'); c.px(15, 21, 'W'); c.px(15, 20, 'J')
    # mantle over the shoulders
    c.poly([(9, 13), (21, 13), (21, 17), (9, 17)], 'J')
    c.rect(16, 13, 21, 17, 'I'); c.rect(9, 17, 21, 17, 'i')
    # hood from behind, its point flopping back
    c.ellipse(15, 9, 4.5, 4.5, 'J', shade='I', hi='W')
    c.poly([(13, 6), (17, 6), (15, 2)], 'J'); c.px(15, 2, 'W')
    c.rect(18, 10, 19, 12, 'i')
    # right arm and staff
    c.rect(21, 15, 22, 18, 'I'); c.px(22, 17, 'i'); c.px(23, 17, 'p')
    c.line(24, 30, 24, 7, 'X'); c.line(23, 30, 23, 8, 'x')
    c.ellipse(24, 4, 2, 2.5, 'J', shade='I', hi='W')
    c.px(27, 3, 'J'); c.px(21, 2, 'J'); c.px(27, 8, 'J')
    return c.outline()


def elowen_back():
    """Elowen Starshot from behind: pale braid over a green cloak, gold
    circlet, quiver of arrows across her back, bow out to the right."""
    c = Canvas()
    # legs
    c.rect(12, 25, 14, 28, 'x'); c.rect(16, 24, 18, 27, 'x')
    c.rect(12, 29, 14, 30, 'C'); c.rect(12, 30, 15, 30, 'B')
    c.rect(16, 28, 18, 29, 'C'); c.rect(16, 29, 19, 29, 'B')
    # cloak
    c.poly([(10, 13), (20, 13), (21, 26), (9, 26)], 'N')
    c.poly([(16, 13), (20, 13), (21, 26), (16, 26)], 'n')
    c.rect(10, 14, 10, 25, 'H'); c.rect(9, 26, 21, 26, 'n')
    # quiver across the back, arrows poking up over the right shoulder
    c.line(11, 23, 18, 13, 'b'); c.line(12, 23, 19, 13, 'C'); c.line(12, 24, 19, 14, 'B')
    c.line(13, 22, 14, 21, 'G')
    c.px(19, 11, 'W'); c.px(20, 12, 'R'); c.px(18, 11, 'R'); c.px(20, 10, 'W'); c.px(21, 11, 'W')
    # head: pale hair, gold circlet, a braid down the back
    c.ellipse(15, 9, 4, 4, 'w', shade='x', hi='W')
    c.rect(11, 8, 19, 8, 'G'); c.px(11, 8, 'g')
    c.rect(14, 13, 15, 19, 'w'); c.rect(15, 13, 15, 19, 'x'); c.px(14, 20, 'G')
    c.px(19, 10, 'p'); c.px(20, 9, 'p')                 # pointed ear
    # bow held out on the right, string towards us
    c.line(24, 4, 26, 9, 'c'); c.line(26, 10, 26, 19, 'c'); c.line(26, 20, 24, 26, 'c')
    c.line(23, 5, 23, 25, 'E')
    c.px(25, 3, 'Y'); c.px(24, 27, 'G')
    c.rect(20, 15, 21, 18, 'n'); c.px(22, 16, 'p')
    return c.outline()


BACKS = {
    "bram_gatewarden": bram_back,
    "mirelle_frostweaver": mirelle_back,
    "elowen_starshot": elowen_back,
}
# The rest of the roster, built from shared parts.
from iso_backs import BACKS as _MORE  # noqa: E402
BACKS.update(_MORE)


def roster(name):
    p = os.path.join(ROSTER, name + ".png")
    if not os.path.exists(p):
        p = os.path.join(ROSTER, "monsters", name + ".png")
    return Image.open(p).convert("RGBA")


def facings(name):
    """The four diagonal facings: (NW, NE, SW, SE) images."""
    front = roster(name)
    back = BACKS[name]().image() if name in BACKS else None
    flip = lambda im: im.transpose(Image.FLIP_LEFT_RIGHT) if im else None
    return flip(back), back, flip(front), front


if __name__ == "__main__":
    for name, f in BACKS.items():
        f().image().save(os.path.join(OUT, name + "_back.png"))
    # facing sheet: one row per character, NW NE SW SE at 4x
    s = 4
    sheet = Image.new("RGBA", (4 * 34 * s, len(BACKS) * 34 * s), (29, 26, 43, 255))
    for row, name in enumerate(BACKS):
        for col, im in enumerate(facings(name)):
            big = im.resize((32 * s, 32 * s), Image.NEAREST)
            sheet.paste(big, (col * 34 * s + s, row * 34 * s + s), big)
    sheet.save(os.path.join(OUT, "facings.png"))
    print(sorted(os.listdir(OUT)))
