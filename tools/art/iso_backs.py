"""Back views (facing north-east) for the rest of the roster, built from shared
parts so proportions match: head around rows 4-14, body 13-24, legs to 30.
`iso.py` imports `BACKS` from here alongside its three hand-tuned ones."""
from shapes import Canvas


# ----------------------------------------------------------------- parts
def legs(c, pants='x', boots='B', dark='b', short=False):
    """Two legs from behind, the right a step ahead (higher)."""
    top = 26 if short else 25
    c.rect(12, top, 14, 28, pants); c.rect(16, top - 1, 18, 27, pants)
    c.rect(12, 29, 14, 30, boots); c.rect(12, 30, 15, 30, dark)
    c.rect(16, 28, 18, 29, boots); c.rect(16, 29, 19, 29, dark)


def torso(c, main, shade, hi, y0=15, y1=24, x0=11, x1=19):
    c.rect(x0, y0, x1, y1, main); c.rect(x1 - 1, y0, x1, y1, shade); c.rect(x0, y0 + 1, x0, y1 - 1, hi)


def robe(c, main, shade, hi, hem, y0=15, wide=1):
    """A long robe widening to the hem at row 30."""
    c.poly([(11, y0), (19, y0), (20 + wide, 30), (9 - wide, 30)], main)
    c.poly([(16, y0), (19, y0), (20 + wide, 30), (16, 30)], shade)
    c.poly([(11, y0), (12, y0), (11 - wide // 2, 30), (9 - wide, 30)], hi)
    c.rect(9 - wide, 30, 20 + wide, 30, hem)


def cloak(c, main, shade, hi, y0=13, y1=26):
    c.poly([(10, y0), (20, y0), (21, y1), (9, y1)], main)
    c.poly([(16, y0), (20, y0), (21, y1), (16, y1)], shade)
    c.rect(10, y0 + 1, 10, y1 - 1, hi); c.rect(9, y1, 21, y1, shade)


def arm(c, sleeve, shade, skin='p', y=15):
    """The right arm reaching forward, hand at the right edge."""
    c.rect(20, y, 21, y + 3, sleeve); c.px(21, y + 1, shade); c.px(22, y + 1, skin)


def hair(c, main, shade, hi, cy=9, r=4.5):
    c.ellipse(15, cy, r, r, main, shade=shade, hi=hi)


def ear(c, skin='p', y=10, pointed=False):
    c.px(20, y, skin)
    if pointed:
        c.px(21, y - 1, skin)


def hood(c, main, shade, hi, point=False):
    c.ellipse(15, 9, 5, 5, main, shade=shade, hi=hi)
    if point:
        c.poly([(13, 6), (17, 6), (15, 2)], main)
    c.rect(10, 12, 20, 14, main); c.rect(17, 12, 20, 14, shade)


def staff(c, x=24, top=4, wood='C', dark='B'):
    c.line(x, 30, x, top + 2, wood); c.line(x - 1, 30, x - 1, top + 3, dark)


def blade_up(c, x0=23, y0=15, x1=28, y1=5, steel='L', edge='M', guard='G'):
    c.line(x0, y0, x1, y1, steel); c.line(x0 + 1, y0, x1 + 1, y1 + 1, edge)
    c.line(x0 - 2, y0 - 1, x0 + 2, y0 + 2, guard)


def bow(c, wood='c', string='E'):
    c.line(24, 4, 26, 9, wood); c.line(26, 10, 26, 19, wood); c.line(26, 20, 24, 26, wood)
    c.line(23, 5, 23, 25, string)


def quiver(c, case='C', dark='b', light='B'):
    c.line(11, 23, 18, 13, dark); c.line(12, 23, 19, 13, case); c.line(12, 24, 19, 14, light)
    c.px(19, 11, 'W'); c.px(20, 12, 'R'); c.px(18, 11, 'R'); c.px(20, 10, 'W')


def shield_back(c, wood='C', shade='B', hi='c', rim='S', tall=False):
    """A shield on the left arm (screen left), its back towards us."""
    if tall:
        c.rect(3, 13, 10, 27, wood); c.rect(3, 13, 3, 27, hi); c.rect(9, 13, 10, 27, shade)
        c.rect(3, 13, 10, 13, rim); c.rect(3, 27, 10, 27, rim)
        c.rect(4, 18, 9, 19, 'b'); c.rect(4, 22, 9, 23, 'b')
    else:
        c.ellipse(7, 19, 4, 5.5, wood, shade=shade, hi=hi)
        c.line(6, 14, 6, 24, shade); c.line(8, 14, 8, 24, shade); c.rect(4, 18, 10, 19, 'b')
    c.rect(9, 16, 10, 20, 'M')


# ----------------------------------------------------------------- characters
def vorn():
    """Archmage Vorn: deep purple robe with gold hem, long white hair, staff
    with a purple gem, a flicker of fire in his left hand."""
    c = Canvas()
    robe(c, 'A', 'v', 'a', 'G')
    c.rect(14, 22, 16, 22, 'G')
    c.ellipse(15, 12, 4, 3, 'A', shade='v', hi='a')                    # hood of the robe
    c.poly([(12, 9), (18, 9), (17, 15), (15, 17), (13, 15)], 'x')       # long white hair
    c.poly([(16, 9), (18, 9), (17, 15), (15, 17)], 'X'); c.line(13, 10, 14, 14, 'w')
    c.poly([(10, 7), (20, 7), (15, 0)], 'A'); c.rect(9, 7, 21, 8, 'v')  # wizard hat
    c.px(14, 3, 'a'); c.px(13, 5, 'a')
    arm(c, 'A', 'v')
    staff(c, top=4, wood='C', dark='B'); c.ellipse(24, 4, 2, 2, 'a', shade='A', hi='W')
    c.px(6, 17, 'F'); c.px(5, 16, 'Y'); c.px(7, 16, 'F'); c.px(6, 15, 'Y')
    c.rect(8, 17, 9, 19, 'A')
    return c.outline()


def calloway():
    """Brother Calloway: brown monk's habit, rope belt, tonsured head, a staff
    hung with a lantern."""
    c = Canvas()
    robe(c, 'B', 'b', 'C', 'b')
    c.rect(11, 20, 19, 20, 'c'); c.line(13, 21, 13, 25, 'c')         # rope belt
    c.ellipse(15, 13, 5, 2.5, 'B', shade='b', hi='C')                  # hood down on shoulders
    hair(c, 'p', 'k', 'P', cy=9, r=4)
    c.rect(11, 9, 19, 12, 'c'); c.rect(17, 9, 19, 12, 'C')             # fringe of hair
    ear(c, y=10)
    arm(c, 'B', 'b')
    staff(c, top=3)
    c.rect(26, 5, 28, 8, 'G'); c.px(27, 6, 'Y'); c.px(27, 4, 'K'); c.px(25, 4, 'C'); c.px(26, 4, 'C')
    return c.outline()


def grub():
    """Grub Ironjaw: a broad orc in leather, bald green head, axe on his
    shoulder."""
    c = Canvas()
    legs(c, 'B', 'b', 'K')
    torso(c, 'B', 'b', 'C', y0=14, x0=10, x1=20)
    c.rect(10, 14, 20, 15, 'o'); c.rect(9, 15, 10, 21, 'O'); c.rect(20, 15, 21, 20, 'o')  # shoulders, arms
    c.line(15, 16, 15, 23, 'b'); c.rect(10, 21, 20, 21, 'K'); c.px(15, 21, 'M')
    hair(c, 'O', 'o', 'q', cy=10, r=4.5)
    c.px(20, 9, 'O'); c.px(21, 8, 'O')                                 # ear
    c.rect(13, 14, 17, 14, 'o')                                        # thick neck
    c.px(22, 16, 'O')
    c.line(23, 18, 26, 4, 'C'); c.line(24, 18, 27, 5, 'B')             # haft
    c.poly([(25, 3), (30, 2), (30, 9), (26, 8)], 'L'); c.rect(29, 2, 30, 9, 'M')
    return c.outline()


def hilda():
    """Hilda Bloodaxe: horned helm, two red braids over mail, a long axe."""
    c = Canvas()
    legs(c, 'B', 'b', 'K')
    torso(c, 'M', 'S', 'L', y0=14)
    c.rect(11, 20, 19, 24, 'B'); c.rect(18, 20, 19, 24, 'b')           # leather skirt
    c.rect(11, 19, 19, 19, 'b'); c.px(15, 19, 'G')
    c.rect(12, 12, 13, 20, 'F'); c.rect(17, 12, 18, 20, 'g')           # braids
    c.px(12, 21, 'R'); c.px(18, 21, 'R')
    hair(c, 'F', 'g', 'Y', cy=10, r=4)
    c.ellipse(15, 7, 4.5, 3, 'L', shade='M', hi='W'); c.rect(10, 9, 20, 9, 'S')
    c.poly([(9, 7), (6, 2), (8, 7)], 'w'); c.poly([(21, 7), (24, 2), (22, 7)], 'x')
    arm(c, 'M', 'S', y=15)
    c.line(24, 30, 24, 3, 'C'); c.line(23, 30, 23, 4, 'B')
    c.poly([(25, 3), (29, 1), (29, 10), (25, 8)], 'L'); c.rect(28, 1, 29, 10, 'M'); c.px(24, 1, 'M')
    return c.outline()


def kestrel():
    """Kestrel: green hooded cloak, quiver, longbow."""
    c = Canvas()
    legs(c, 'B', 'b', 'K')
    cloak(c, 'N', 'n', 'H')
    quiver(c)
    hood(c, 'N', 'n', 'H')
    arm(c, 'n', 'N')
    bow(c)
    return c.outline()


def korgath():
    """Korgath the Undying: dark spiked plate, grey skin, a heavy mace."""
    c = Canvas()
    legs(c, 'D', 'd', 'K')
    torso(c, 'D', 'd', 'e', y0=14, x0=10, x1=20)
    c.rect(9, 13, 12, 15, 'e'); c.rect(18, 13, 21, 15, 'D')            # pauldrons
    c.px(10, 12, 'E'); c.px(20, 12, 'e')                               # spikes
    c.line(15, 16, 15, 23, 'd'); c.rect(10, 21, 20, 21, 'r')           # red sash
    hair(c, 'E', 'e', 'w', cy=9, r=4)                                  # pale bald skull
    c.rect(12, 12, 18, 13, 'e')
    c.px(13, 7, 'D'); c.px(17, 9, 'D')                                 # scars
    arm(c, 'D', 'd', skin='E', y=16)
    c.line(23, 20, 26, 7, 'C'); c.line(24, 20, 27, 8, 'B')
    c.ellipse(27, 5, 3, 3, 'e', shade='D', hi='E'); c.px(30, 5, 'E'); c.px(27, 2, 'E'); c.px(24, 5, 'E')
    return c.outline()


def nettle():
    """Nettle: a small hooded rogue, crouched, dagger held out."""
    c = Canvas()
    legs(c, 'b', 'K', 'K', short=True)
    c.rect(12, 26, 18, 26, 'b')
    cloak(c, 'B', 'b', 'C', y0=15, y1=26)
    c.ellipse(15, 12, 4.5, 4.5, 'B', shade='b', hi='C')                # hood, head low
    c.rect(11, 15, 19, 16, 'B'); c.rect(17, 15, 19, 16, 'b')
    arm(c, 'B', 'b', skin='O', y=18)
    c.line(23, 19, 27, 16, 'L'); c.px(22, 19, 'G')
    return c.outline()


def pip():
    """Pip Quickfeather: a halfling in green, feathered cap, brown curls, a
    short bow."""
    c = Canvas()
    legs(c, 'B', 'C', 'b', short=True)
    torso(c, 'N', 'n', 'H', y0=16, y1=25)
    c.rect(11, 22, 19, 22, 'B')
    quiver(c)
    hair(c, 'B', 'b', 'C', cy=11, r=4.5)
    c.ellipse(15, 8, 5, 2.5, 'N', shade='n', hi='H'); c.rect(9, 9, 21, 9, 'n')  # cap
    c.line(17, 7, 22, 2, 'R'); c.line(18, 7, 22, 3, 'r')               # feather
    ear(c, y=12, pointed=True)
    arm(c, 'N', 'n', y=17)
    c.line(24, 8, 26, 12, 'c'); c.line(26, 13, 26, 20, 'c'); c.line(26, 21, 24, 26, 'c')
    c.line(23, 9, 23, 25, 'E')
    return c.outline()


def seraphine():
    """Seraphine Dawnbringer: white and gold robe, golden hair, a halo and
    white wings folded on her back, a sun staff."""
    c = Canvas()
    robe(c, 'w', 'x', 'W', 'G')
    c.rect(13, 20, 17, 20, 'G')
    # wings
    c.poly([(14, 13), (5, 9), (3, 15), (7, 24), (13, 22)], 'W')
    c.poly([(16, 13), (25, 9), (26, 15), (23, 24), (17, 22)], 'x')
    c.line(6, 12, 12, 18, 'x'); c.line(5, 16, 11, 21, 'x'); c.line(24, 12, 18, 18, 'X'); c.line(24, 16, 19, 21, 'X')
    hair(c, 'Y', 'G', 'W', cy=9, r=4)
    c.rect(12, 12, 18, 16, 'Y'); c.rect(16, 12, 18, 16, 'G')
    c.ellipse(15, 3, 4, 1, 'Y'); c.ellipse(15, 3, 2, 0.4, '.') if False else None
    c.rect(13, 3, 17, 3, '.'); c.px(11, 3, 'Y'); c.px(19, 3, 'Y'); c.rect(12, 2, 18, 2, 'Y'); c.rect(12, 4, 18, 4, 'Y')
    c.rect(13, 3, 17, 3, '.')
    staff(c, x=27, top=5, wood='G', dark='g')
    c.ellipse(27, 4, 2, 2, 'Y', hi='W'); c.px(27, 1, 'Y'); c.px(30, 4, 'Y'); c.px(24, 4, 'Y')
    return c.outline()


def aldric():
    """Sir Aldric: full plate, plumed helm, a tall shield (its back to us) and
    a spear."""
    c = Canvas()
    legs(c, 'M', 'S', 'D')
    torso(c, 'L', 'M', 'W', y0=14)
    c.rect(11, 21, 19, 22, 'M'); c.line(15, 15, 15, 20, 'M')
    c.ellipse(15, 9, 4.5, 4.5, 'L', shade='M', hi='W'); c.rect(11, 12, 19, 13, 'M')
    c.line(15, 4, 15, 9, 'R'); c.line(16, 3, 18, 7, 'R'); c.line(14, 4, 16, 2, 'r')  # plume
    shield_back(c, tall=True)
    arm(c, 'L', 'M', skin='M', y=15)
    c.line(23, 30, 23, 2, 'C'); c.line(24, 29, 24, 3, 'B'); c.poly([(22, 3), (24, 3), (23, 0)], 'L')
    return c.outline()


def agnes():
    """Sister Agnes: black habit with a white wimple edge, a prayer book."""
    c = Canvas()
    robe(c, 'D', 'd', 'e', 'd', y0=14)
    c.ellipse(15, 9, 5, 5, 'D', shade='d', hi='e')
    c.rect(10, 11, 20, 17, 'D'); c.rect(17, 11, 20, 17, 'd'); c.rect(10, 12, 10, 16, 'e')  # veil
    c.rect(10, 18, 20, 18, 'W'); c.rect(9, 7, 9, 13, 'W'); c.rect(10, 5, 11, 6, 'W')  # wimple
    c.line(15, 19, 15, 29, 'd')
    arm(c, 'd', 'K', y=16)
    c.rect(22, 15, 25, 18, 'r'); c.rect(22, 15, 25, 15, 'R'); c.px(24, 17, 'G')
    return c.outline()


def widow():
    """The Grey Widow: tall grey hood and cloak, a crossbow."""
    c = Canvas()
    robe(c, 'E', 'e', 'L', 'D', y0=13, wide=2)
    hood(c, 'E', 'e', 'L', point=True)
    c.rect(15, 15, 15, 29, 'e')
    arm(c, 'E', 'e', skin='D', y=15)
    c.line(23, 9, 23, 21, 'B'); c.line(21, 12, 27, 12, 'C'); c.line(20, 13, 21, 12, 'L'); c.line(27, 12, 28, 13, 'L')
    c.line(23, 9, 23, 7, 'L')
    return c.outline()


def tobble():
    """Tobble the Apprentice: small, purple robe, a big floppy brown hat with
    a gold star, a wand with a spark."""
    c = Canvas()
    robe(c, 'A', 'v', 'a', 'v', y0=17)
    hair(c, 'B', 'b', 'C', cy=13, r=4)
    c.poly([(8, 12), (22, 12), (17, 3), (13, 3)], 'C'); c.poly([(16, 3), (22, 12), (17, 12)], 'B')
    c.poly([(13, 3), (17, 3), (18, 0)], 'C'); c.rect(7, 12, 23, 13, 'B')
    c.px(14, 8, 'G'); c.px(13, 9, 'G'); c.px(15, 9, 'G'); c.px(14, 10, 'G')
    arm(c, 'A', 'v', y=18)
    c.line(23, 19, 26, 14, 'C'); c.px(27, 13, 'F'); c.px(28, 12, 'Y'); c.px(26, 12, 'Y'); c.px(28, 14, 'Y')
    return c.outline()


def vex():
    """Vex: purple hair, dark leathers, a short purple cape, dagger out."""
    c = Canvas()
    legs(c, 'd', 'K', 'K')
    torso(c, 'd', 'K', 'D', y0=15)
    c.poly([(10, 14), (20, 14), (20, 21), (10, 22)], 'A'); c.poly([(16, 14), (20, 14), (20, 21), (16, 21)], 'v')
    c.rect(11, 22, 19, 22, 'B')
    hair(c, 'A', 'v', 'a', cy=9, r=4.5)
    c.rect(12, 12, 18, 14, 'A'); c.rect(16, 12, 18, 14, 'v')
    c.line(10, 6, 7, 10, 'a')                                          # flick of hair
    ear(c, y=10)
    arm(c, 'd', 'K', y=15)
    c.line(23, 16, 27, 13, 'L'); c.px(22, 16, 'G')
    return c.outline()


def ysolde():
    """Ysolde the Unbroken: silver armour, purple cape, blonde hair, shield
    on her left arm, sword raised."""
    c = Canvas()
    legs(c, 'M', 'S', 'D')
    cloak(c, 'A', 'v', 'a', y0=14, y1=27)
    c.rect(11, 13, 19, 14, 'L'); c.rect(18, 13, 19, 14, 'M')
    hair(c, 'Y', 'G', 'W', cy=9, r=4.5)
    c.rect(12, 12, 18, 13, 'Y')
    shield_back(c, wood='E', shade='e', hi='L')
    arm(c, 'L', 'M', skin='M', y=15)
    blade_up(c)
    return c.outline()


BACKS = {
    "archmage_vorn": vorn,
    "brother_calloway": calloway,
    "grub_ironjaw": grub,
    "hilda_bloodaxe": hilda,
    "kestrel": kestrel,
    "korgath_the_undying": korgath,
    "nettle": nettle,
    "pip_quickfeather": pip,
    "seraphine_dawnbringer": seraphine,
    "sir_aldric": aldric,
    "sister_agnes": agnes,
    "the_grey_widow": widow,
    "tobble_the_apprentice": tobble,
    "vex": vex,
    "ysolde_the_unbroken": ysolde,
}
