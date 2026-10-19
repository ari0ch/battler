"""Shape-based pixel art: fill shapes with palette colours on a small canvas,
then add a 1px dark outline automatically. Used for the perspective study."""
from PIL import Image
from engine import PAL

class Canvas:
    def __init__(self, w=32, h=32):
        self.w, self.h = w, h
        self.g = [['.'] * w for _ in range(h)]

    def px(self, x, y, c):
        x, y = int(round(x)), int(round(y))
        if 0 <= x < self.w and 0 <= y < self.h:
            self.g[y][x] = c

    def rect(self, x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.px(x, y, c)

    def ellipse(self, cx, cy, rx, ry, c, shade=None, hi=None):
        """Filled ellipse; optional darker lower-right and lighter upper-left."""
        for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
            for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
                dx, dy = (x - cx) / max(rx, .5), (y - cy) / max(ry, .5)
                d = dx * dx + dy * dy
                if d <= 1.0:
                    col = c
                    if shade and dx + dy > 0.55:
                        col = shade
                    if hi and dx + dy < -0.85:
                        col = hi
                    self.px(x, y, col)

    def poly(self, pts, c):
        ys = [p[1] for p in pts]
        for y in range(int(min(ys)), int(max(ys)) + 1):
            xs = []
            n = len(pts)
            for i in range(n):
                (x0, y0), (x1, y1) = pts[i], pts[(i + 1) % n]
                if (y0 <= y + .5 < y1) or (y1 <= y + .5 < y0):
                    xs.append(x0 + (y + .5 - y0) * (x1 - x0) / (y1 - y0))
            xs.sort()
            for i in range(0, len(xs) - 1, 2):
                for x in range(int(round(xs[i])), int(round(xs[i + 1]))):
                    self.px(x, y, c)

    def line(self, x0, y0, x1, y1, c):
        n = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        for i in range(n + 1):
            t = i / n
            self.px(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, c)

    def outline(self, c='K'):
        """Every clear pixel touching a filled one (4-neighbour) becomes outline."""
        add = []
        for y in range(self.h):
            for x in range(self.w):
                if self.g[y][x] != '.':
                    continue
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    X, Y = x + dx, y + dy
                    if 0 <= X < self.w and 0 <= Y < self.h and self.g[Y][X] not in ('.', c):
                        add.append((x, y)); break
        for x, y in add:
            self.g[y][x] = c
        return self

    def image(self):
        img = Image.new("RGBA", (self.w, self.h), (0, 0, 0, 0))
        for y, row in enumerate(self.g):
            for x, ch in enumerate(row):
                if ch != '.':
                    img.putpixel((x, y), PAL[ch] + (255,))
        return img
