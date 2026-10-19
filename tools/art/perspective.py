"""Example sprites for the perspective study, drawn with shapes + auto outline."""
import os
from shapes import Canvas

OUT = os.path.join(os.path.dirname(__file__), "out", "persp"); os.makedirs(OUT, exist_ok=True)

# ---------------------------------------------------------------- 3/4 view (high angle)
def knight_back():
    """Bram from behind, walking up the screen: helm dome, cloak, shield on his left."""
    c = Canvas()
    # boots / legs
    c.rect(12, 26, 14, 30, 'b'); c.rect(17, 26, 19, 30, 'b')
    c.rect(12, 25, 14, 27, 'M'); c.rect(17, 25, 19, 27, 'S')
    # body: brown tabard over mail, seen from behind
    c.ellipse(15.5, 19.5, 6.5, 6.5, 'B', shade='b', hi='C')
    c.rect(10, 15, 21, 23, 'B'); c.rect(18, 15, 21, 23, 'b'); c.rect(10, 15, 11, 22, 'C')
    c.rect(10, 22, 21, 24, 'M'); c.rect(15, 22, 21, 24, 'S')  # mail skirt
    c.line(15, 15, 15, 23, 'b')                                 # tabard seam
    # arms
    c.rect(8, 16, 9, 22, 'M'); c.rect(22, 16, 23, 21, 'S')
    # shield on his left arm (screen left), seen from the back: wooden boards
    c.ellipse(7, 19, 4, 5, 'C', shade='B', hi='c'); c.line(7, 14, 7, 24, 'b'); c.line(4, 19, 10, 19, 'b')
    # sword in his right hand (screen right), pointing up and out
    c.line(24, 20, 27, 9, 'L'); c.line(25, 20, 28, 10, 'M'); c.rect(22, 20, 25, 21, 'G')
    # helm from behind: a steel dome with a brim, coif at the neck
    c.ellipse(15.5, 9, 5, 5, 'L', shade='M', hi='W')
    c.rect(10, 11, 21, 12, 'S'); c.rect(11, 13, 20, 14, 'M')
    return c.outline()

def knight_front():
    """Bram facing down the screen (for enemies or turning around)."""
    c = Canvas()
    c.rect(12, 26, 14, 30, 'b'); c.rect(17, 26, 19, 30, 'b')
    c.rect(12, 25, 14, 27, 'M'); c.rect(17, 25, 19, 27, 'S')
    c.rect(10, 15, 21, 23, 'B'); c.rect(18, 15, 21, 23, 'b'); c.rect(10, 15, 11, 22, 'C')
    c.rect(13, 15, 18, 22, 'C'); c.rect(14, 18, 17, 19, 'G')  # tabard with gold band
    c.rect(10, 22, 21, 24, 'M'); c.rect(15, 22, 21, 24, 'S')
    c.rect(8, 16, 9, 22, 'M'); c.rect(22, 16, 23, 21, 'S')
    # shield on his left arm, now on screen right, painted face
    c.ellipse(24, 19, 4, 5, 'c', shade='C', hi='P'); c.ellipse(24, 19, 1.5, 1.5, 'M')
    # sword on screen left
    c.line(7, 20, 5, 9, 'L'); c.line(6, 20, 4, 10, 'M'); c.rect(6, 20, 9, 21, 'G')
    # helm and face
    c.ellipse(15.5, 9, 5, 5, 'L', shade='M', hi='W')
    c.rect(11, 9, 20, 13, 'p'); c.rect(11, 9, 20, 9, 'S')
    c.px(13, 11, 'K'); c.px(18, 11, 'K'); c.rect(14, 13, 17, 13, 'k')
    c.rect(10, 8, 21, 8, 'S')
    return c.outline()

def goblin_front():
    """A goblin brute facing down the screen, club raised."""
    c = Canvas()
    c.rect(11, 26, 14, 30, 'o'); c.rect(17, 26, 20, 30, 'o')
    c.rect(11, 29, 14, 30, 'b'); c.rect(17, 29, 20, 30, 'b')
    c.rect(10, 17, 21, 26, 'B'); c.rect(18, 17, 21, 26, 'b'); c.rect(13, 19, 18, 24, 'C')
    c.ellipse(8, 20, 2.5, 4, 'O', shade='o'); c.ellipse(23, 20, 2.5, 4, 'O', shade='o')
    # club in the right hand (screen left), raised
    c.line(7, 19, 4, 8, 'C'); c.ellipse(4, 7, 2.5, 3, 'B', shade='b')
    # big head with ears
    c.ellipse(15.5, 11, 6.5, 5.5, 'O', shade='o', hi='q')
    c.poly([(9, 9), (3, 6), (9, 13)], 'O'); c.poly([(22, 9), (28, 6), (22, 13)], 'o')
    c.rect(12, 10, 13, 11, 'Y'); c.rect(18, 10, 19, 11, 'Y'); c.px(13, 11, 'K'); c.px(18, 11, 'K')
    c.rect(13, 14, 18, 14, 'K'); c.px(14, 15, 'W'); c.px(17, 15, 'W')
    return c.outline()

# ---------------------------------------------------------------- true top-down
def knight_top():
    """Bram seen from straight above, facing up: helm, shoulders, shield, sword."""
    c = Canvas()
    c.ellipse(15.5, 18, 8, 5, 'B', shade='b', hi='C')        # shoulders / tabard
    c.ellipse(7.5, 16, 4, 4.5, 'C', shade='B', hi='c'); c.ellipse(7.5, 16, 1.2, 1.2, 'M')  # shield
    c.line(23, 18, 24, 6, 'L'); c.line(24, 18, 25, 6, 'M'); c.rect(22, 17, 25, 18, 'G')   # sword up
    c.ellipse(15.5, 16.5, 4, 4, 'L', shade='M', hi='W')       # helm from above
    c.rect(15, 13, 16, 20, 'S')                                # helm crest ridge
    return c.outline()

def goblin_top():
    c = Canvas()
    c.ellipse(15.5, 18, 7, 4.5, 'B', shade='b', hi='C')
    c.line(8, 16, 7, 7, 'C'); c.ellipse(7, 6, 2.5, 2.5, 'B', shade='b')
    c.ellipse(15.5, 17, 4.5, 4, 'O', shade='o', hi='q')
    c.poly([(11, 15), (6, 12), (11, 18)], 'O'); c.poly([(20, 15), (25, 12), (20, 18)], 'o')
    c.px(14, 19, 'Y'); c.px(17, 19, 'Y')
    return c.outline()

# ---------------------------------------------------------------- isometric ground tile
def iso_tile(base='grass1'):
    """A 32x16 isometric diamond of grass with a lighter top edge."""
    from kit import C, new, px, save, OUT as UI
    img = new(32, 17)
    for y in range(17):
        half = 16 - abs(8 - y) * 2
        for x in range(16 - half, 16 + half):
            px(img, x, y, base if (x + y) % 7 else 'grass2')
    for x in range(32):  # edges
        y = 8 - abs(x - 15.5) / 2
        px(img, x, int(round(8 - abs(x - 15.5) / 2)), 'grass0')
        px(img, x, int(round(8 + abs(x - 15.5) / 2)), 'grass0')
    img.save(os.path.join(OUT, "iso_tile.png"))
    return img

SPRITES = {
    "knight_back": knight_back, "knight_front": knight_front, "goblin_front": goblin_front,
    "knight_top": knight_top, "goblin_top": goblin_top,
}
if __name__ == "__main__":
    for name, f in SPRITES.items():
        f().image().save(os.path.join(OUT, name + ".png"))
    iso_tile()
    print(sorted(os.listdir(OUT)))
