"""Draws Aerie's block atlas: one 16x16 tile per block layer, stacked
vertically into assets/blocks.png. Every tile is generated here from a fixed
seed and a pastel palette, so the art is ours and reproducible.

Tiles are built from a few shared pieces: colour ramps (dark to light),
tileable smooth noise for clumps instead of per-pixel speckle, and
tileable Voronoi cells for stones. Light comes from the top left, so
shapes get a light upper edge and a dark lower one.

Run: python3 tools/make_atlas.py   (from examples/aerie)
"""
import math
import random
from PIL import Image

S = 16
CLEAR = (0, 0, 0, 0)

# Colour ramps, dark to light: dreamy pastels.
STONE = [(118, 112, 150), (146, 140, 178), (172, 167, 202), (198, 194, 222), (224, 220, 240)]
LOAM = [(122, 84, 102), (150, 106, 120), (176, 130, 138), (200, 156, 156)]
MOSS = [(78, 160, 146), (104, 188, 162), (138, 214, 178), (178, 236, 198), (214, 250, 222)]
BARK = [(116, 64, 90), (148, 86, 108), (180, 110, 124), (208, 140, 146)]
WOOD = [(222, 168, 150), (240, 196, 172), (252, 222, 196)]
BLOSSOM = [(196, 70, 150), (232, 104, 178), (250, 142, 200), (255, 182, 220), (255, 222, 240)]
GLASS = [(196, 220, 250), (220, 236, 255), (246, 250, 255)]
IRON = [(70, 92, 210), (108, 140, 246), (160, 190, 255), (226, 238, 255)]
CAP = [(64, 186, 186), (104, 220, 210), (160, 244, 228), (232, 255, 248)]
STEM = [(214, 198, 186), (240, 230, 216), (255, 250, 240)]
SAIL = [(214, 192, 170), (232, 214, 192), (246, 234, 214), (255, 248, 234)]
STRAW = [(206, 162, 96), (230, 194, 120), (246, 222, 150), (255, 240, 190)]
PLANK = [(178, 104, 112), (206, 132, 130), (228, 160, 148), (244, 188, 170)]
FLOWERS = [(255, 176, 214), (255, 250, 240), (206, 180, 255), (255, 226, 150)]


def ramp(colors, v):
    """The colour at `v` (0 to 1) along a ramp, picked, not blended, so the
    tile keeps a small pixel-art palette."""
    i = int(max(0.0, min(0.999, v)) * len(colors))
    return colors[i]


def smooth_noise(rng, cells):
    """Tileable value noise: a random `cells`x`cells` grid, smoothly
    interpolated up to S x S. Returns rows of 0..1."""
    grid = [[rng.random() for _ in range(cells)] for _ in range(cells)]
    out = []
    for y in range(S):
        row = []
        for x in range(S):
            gx, gy = x * cells / S, y * cells / S
            x0, y0 = int(gx), int(gy)
            fx, fy = gx - x0, gy - y0
            fx, fy = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy)
            a = grid[y0 % cells][x0 % cells]
            b = grid[y0 % cells][(x0 + 1) % cells]
            c = grid[(y0 + 1) % cells][x0 % cells]
            d = grid[(y0 + 1) % cells][(x0 + 1) % cells]
            row.append((a * (1 - fx) + b * fx) * (1 - fy) + (c * (1 - fx) + d * fx) * fy)
        out.append(row)
    return out


def fbm(rng, weights=((2, 0.5), (4, 0.3), (8, 0.2))):
    """A few octaves of smooth noise, each `(cells, weight)`."""
    layers = [(smooth_noise(rng, c), w) for c, w in weights]
    return [[sum(l[y][x] * w for l, w in layers) for x in range(S)] for y in range(S)]


def voronoi(points):
    """For each pixel: (nearest point index, distance to it, gap to the
    second nearest), wrapping at the tile edges."""
    out = []
    for y in range(S):
        row = []
        for x in range(S):
            ds = []
            for i, (px, py) in enumerate(points):
                dx = min(abs(x + 0.5 - px), S - abs(x + 0.5 - px))
                dy = min(abs(y + 0.5 - py), S - abs(y + 0.5 - py))
                ds.append((math.hypot(dx, dy), i))
            ds.sort()
            row.append((ds[0][1], ds[0][0], ds[1][0] - ds[0][0]))
        out.append(row)
    return out


def blank(color=CLEAR):
    return [[color] * S for _ in range(S)]


def stones(rng, colors, count, mortar):
    """Rounded stones: Voronoi cells, each its own shade, lit from the top
    left, with dark mortar where cells meet."""
    pts = [(rng.uniform(0, S), rng.uniform(0, S)) for _ in range(count)]
    shade = [rng.uniform(0.4, 0.7) for _ in pts]
    cells = voronoi(pts)
    grain = fbm(rng, ((8, 1.0),))
    t = blank()
    for y in range(S):
        for x in range(S):
            i, d, gap = cells[y][x]
            if gap < 1.0:
                t[y][x] = mortar
                continue
            # Toward the cell's top left is lighter, the bottom right darker.
            px, py = pts[i]
            dx = ((x + 0.5 - px + S / 2) % S) - S / 2
            dy = ((y + 0.5 - py + S / 2) % S) - S / 2
            light = -(dx + dy) * 0.015
            edge = -0.2 if gap < 2.0 and (dx + dy) > 0 else (0.22 if gap < 2.0 else 0.0)
            t[y][x] = ramp(colors, shade[i] + light + edge + (grain[y][x] - 0.5) * 0.15)
    return t


def driftstone(rng):
    return stones(rng, STONE, 7, STONE[0])


def loam(rng):
    n = fbm(rng)
    t = [[ramp(LOAM, n[y][x] * 1.1 - 0.05) for x in range(S)] for y in range(S)]
    # Small pebbles: a light pixel with a dark one under it.
    for _ in range(4):
        x, y = rng.randrange(S), rng.randrange(S - 1)
        t[y][x] = STONE[3]
        t[y + 1][x] = LOAM[0]
    return t


def moss_top(rng):
    n = fbm(rng)
    t = [[ramp(MOSS, n[y][x] * 1.0) for x in range(S)] for y in range(S)]
    # Blades: short light strokes with a dark root.
    for _ in range(22):
        x, y = rng.randrange(S), rng.randrange(S)
        t[y][x] = MOSS[4] if rng.random() < 0.5 else MOSS[3]
        t[(y + 1) % S][x] = MOSS[1]
    # One flower per tile: every block repeats the tile, so more would
    # read as a grid.
    for _ in range(1):
        x, y = rng.randrange(S), rng.randrange(S)
        petal = rng.choice(FLOWERS[:3])
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            t[(y + dy) % S][(x + dx) % S] = petal
        t[y][x] = FLOWERS[3]
    return t


def moss_side(rng):
    t = loam(rng)
    for x in range(S):
        depth = 3 + rng.choice([0, 0, 1, 1, 2, 3])
        for y in range(depth):
            v = 1.0 - y / (depth + 1)
            t[y][x] = ramp(MOSS, 0.25 + v * 0.7 + rng.uniform(-0.1, 0.1))
        t[depth][x] = MOSS[0]
    return t


def bark(rng):
    t = blank()
    n = fbm(rng, ((2, 0.6), (8, 0.4)))
    for x in range(S):
        # Ridges every 4 columns: light on the left of a ridge, dark groove.
        phase = x % 4
        base = [0.45, 0.75, 0.55, 0.1][phase]
        for y in range(S):
            wobble = (n[y][x] - 0.5) * 0.4
            t[y][x] = ramp(BARK, base + wobble)
    # Knots break up the grooves.
    for _ in range(2):
        x, y = rng.randrange(S), rng.randrange(S)
        t[y][x] = BARK[0]
        t[(y + 1) % S][x] = BARK[1]
    return t


def rings(rng):
    t = blank()
    c = 7.5
    for y in range(S):
        for x in range(S):
            if x in (0, S - 1) or y in (0, S - 1):
                t[y][x] = BARK[1] if (x + y) % 3 else BARK[0]
                continue
            d = math.hypot(x - c, y - c)
            t[y][x] = WOOD[0] if int(d * 0.8 + 0.5) % 2 == 0 else (WOOD[1] if d > 2 else WOOD[2])
    t[7][7] = t[8][8] = t[7][8] = t[8][7] = WOOD[0]
    return t


def emberleaf(rng):
    """Cotton candy blossoms: round clusters, each lit from the top left,
    with a few holes for the alpha test."""
    n = fbm(rng, ((4, 0.7), (8, 0.3)))
    t = [[ramp(BLOSSOM, 0.1 + n[y][x] * 0.5) for x in range(S)] for y in range(S)]
    for _ in range(9):
        cx, cy = rng.randrange(S), rng.randrange(S)
        r = rng.uniform(1.6, 2.6)
        for dy in range(-3, 4):
            for dx in range(-3, 4):
                d = math.hypot(dx, dy)
                if d <= r:
                    light = 0.65 + (-(dx + dy) / (2 * r)) * 0.35
                    t[(cy + dy) % S][(cx + dx) % S] = ramp(BLOSSOM, light)
    for _ in range(14):
        t[rng.randrange(S)][rng.randrange(S)] = CLEAR
    for _ in range(3):  # glints
        t[rng.randrange(S)][rng.randrange(S)] = BLOSSOM[4]
    return t


def cloudglass(rng):
    n = fbm(rng, ((4, 1.0),))
    t = [[(*ramp(GLASS, n[y][x]), 90 + int(n[y][x] * 40)) for x in range(S)] for y in range(S)]
    for i in range(S):
        t[0][i] = t[i][0] = (*GLASS[2], 230)
        t[S - 1][i] = t[i][S - 1] = (*GLASS[0], 230)
    for i in range(3):  # a diagonal shine
        t[3 + i][5 - i] = (255, 255, 255, 210)
        t[4 + i][7 - i] = (255, 255, 255, 160)
    t[11][11] = t[12][12] = (255, 255, 255, 180)
    return t


def skyiron(rng):
    t = driftstone(rng)
    # Crystals: diamonds with a lit upper left, a dark lower right and a
    # white glint.
    for x, y in ((4, 4), (11, 9), (5, 12)):
        x, y = x + rng.randrange(-1, 2), y + rng.randrange(-1, 2)
        for dy in range(-2, 3):
            for dx in range(-2, 3):
                if abs(dx) + abs(dy) > 2:
                    continue
                facet = 2 if dx + dy < 0 else (1 if dx + dy == 0 else 0)
                t[(y + dy) % S][(x + dx) % S] = IRON[facet]
        t[(y - 1) % S][(x - 1) % S] = IRON[3]
    return t


def glowcap(rng):
    t = blank()
    for y in range(9, S):
        for x in (7, 8):
            t[y][x] = STEM[2] if x == 7 else STEM[1]
    t[15][6] = t[15][9] = STEM[0]
    for y in range(3, 10):
        w = min(y - 1, 6)
        for x in range(8 - w, 8 + w):
            light = 0.9 - (y - 3) * 0.1 - (x - 4) * 0.03
            t[y][x] = ramp(CAP, light)
    for x, y in ((6, 4), (9, 5), (4, 7), (11, 7), (7, 7)):  # spots
        t[y][x] = CAP[3]
    for x in range(2, 14):  # the dark rim under the cap
        t[9][x] = CAP[0]
    return t


def sailcloth(rng):
    t = blank()
    for y in range(S):
        for x in range(S):
            weave = (x // 2 + y // 2) % 2
            t[y][x] = SAIL[1 + weave] if (x + y) % 2 else SAIL[weave]
    for x in range(S):  # stitched hems
        t[1][x] = SAIL[0]
        t[14][x] = SAIL[0]
        if x % 3 == 1:
            t[2][x] = t[13][x] = BLOSSOM[1]
    return t


def featherthatch(rng):
    """Straw bundles laid in rows, each stroke light at its tip."""
    t = blank(STRAW[0])
    for row in range(4):
        y0 = row * 4
        for x in range(S):
            length = 3 + rng.choice([0, 1])
            for i in range(length):
                y = (y0 + i + (x + row) % 2) % S
                t[y][x] = ramp(STRAW, 0.3 + i / length * 0.7 + rng.uniform(-0.1, 0.1))
        for x in range(S):
            t[(y0 + 3) % S][x] = STRAW[0] if rng.random() < 0.8 else STRAW[1]
    return t


def planks(rng):
    t = blank()
    n = fbm(rng, ((2, 0.5), (16, 0.5)))
    for y in range(S):
        board = y // 4
        for x in range(S):
            if y % 4 == 3:
                t[y][x] = PLANK[0]
            elif y % 4 == 0:
                t[y][x] = PLANK[3]
            else:
                grain = 0.35 + (n[y][(x + board * 5) % S] - 0.5) * 0.6 + (board % 2) * 0.1
                t[y][x] = ramp(PLANK, grain)
    for board in range(4):  # a seam per board, staggered, with nails
        x = (board * 7 + 3) % S
        for y in range(board * 4, board * 4 + 3):
            t[y][x] = PLANK[0]
        t[board * 4 + 1][(x + 2) % S] = STONE[1]
        t[board * 4 + 1][(x - 2) % S] = STONE[1]
    return t


# Layer order is the source of truth for block faces; keep it in sync with
# `Layer` in src/blocks.ks.
TILES = [driftstone, loam, moss_top, moss_side, bark, rings, emberleaf, cloudglass,
         skyiron, glowcap, sailcloth, featherthatch, planks]


def main():
    img = Image.new("RGBA", (S, S * len(TILES)))
    for i, make in enumerate(TILES):
        rng = random.Random(2000 + i)
        tile = make(rng)
        for y in range(S):
            for x in range(S):
                c = tile[y][x]
                img.putpixel((x, i * S + y), c if len(c) == 4 else (*c, 255))
    img.save("assets/blocks.png")
    print(f"wrote assets/blocks.png ({len(TILES)} tiles)")


if __name__ == "__main__":
    main()
