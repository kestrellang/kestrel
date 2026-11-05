"""Draws Aerie's block atlas: one 16x16 tile per block layer, stacked
vertically into assets/blocks.png. Every tile is generated here from a fixed
seed and a small pastel palette, so the art is ours and reproducible.

Run: python3 tools/make_atlas.py   (from examples/aerie)
"""
import random
from PIL import Image

S = 16

# The palette: soft, airy colours. Each block picks from a few of these.
P = {
    "stone0": (196, 198, 210), "stone1": (172, 176, 192), "stone2": (148, 150, 172), "stone3": (214, 216, 226),
    "loam0": (176, 132, 120), "loam1": (152, 110, 104), "loam2": (198, 156, 138),
    "moss0": (138, 196, 150), "moss1": (110, 172, 134), "moss2": (168, 218, 166),
    "bark0": (196, 104, 96), "bark1": (164, 82, 84), "bark2": (222, 134, 116),
    "ring0": (238, 196, 156), "ring1": (214, 160, 128),
    "leaf0": (246, 168, 140), "leaf1": (232, 140, 128), "leaf2": (252, 200, 160),
    "glass0": (226, 240, 252), "glass1": (198, 222, 246),
    "iron0": (110, 150, 226), "iron1": (150, 186, 244),
    "cap0": (180, 236, 220), "cap1": (120, 214, 200), "stem": (240, 232, 214),
    "sail0": (240, 226, 196), "sail1": (218, 198, 166),
    "straw0": (232, 204, 136), "straw1": (206, 172, 108), "straw2": (246, 226, 168),
    "plank0": (226, 148, 120), "plank1": (196, 116, 100),
}

def noise_tile(rng, colors, weights):
    return [[rng.choices(colors, weights)[0] for _ in range(S)] for _ in range(S)]

def driftstone(rng):
    t = noise_tile(rng, [P["stone0"], P["stone1"], P["stone3"]], [6, 3, 2])
    # Soft horizontal strata: islands are layered sediment.
    for y in (4, 11):
        for x in range(S):
            if rng.random() < 0.8:
                t[y][x] = P["stone2"]
    return t

def loam(rng):
    t = noise_tile(rng, [P["loam0"], P["loam1"], P["loam2"]], [6, 3, 2])
    for _ in range(5):  # pebbles
        x, y = rng.randrange(S), rng.randrange(S)
        t[y][x] = P["stone3"]
    return t

def moss_top(rng):
    t = noise_tile(rng, [P["moss0"], P["moss1"], P["moss2"]], [6, 3, 2])
    for _ in range(4):  # tiny white clover flowers
        x, y = rng.randrange(S), rng.randrange(S)
        t[y][x] = (250, 248, 236)
    return t

def moss_side(rng):
    t = loam(rng)
    for x in range(S):
        drip = 3 + rng.choice([0, 0, 1, 1, 2, 3])
        for y in range(drip):
            t[y][x] = rng.choice([P["moss0"], P["moss1"], P["moss2"]])
    return t

def bark(rng):
    t = [[P["bark0"]] * S for _ in range(S)]
    for x in range(S):
        if x % 4 == 1:
            for y in range(S):
                t[y][x] = P["bark1"]
        if x % 4 == 3:
            for y in range(S):
                if rng.random() < 0.5:
                    t[y][x] = P["bark2"]
    return t

def rings(rng):
    t = [[P["bark0"]] * S for _ in range(S)]
    c = 7.5
    for y in range(1, S - 1):
        for x in range(1, S - 1):
            d = ((x - c) ** 2 + (y - c) ** 2) ** 0.5
            t[y][x] = P["ring1"] if int(d) % 3 == 0 else P["ring0"]
    return t

def leaves(rng):
    # Alpha-tested foliage: holes are fully transparent.
    t = [[None] * S for _ in range(S)]
    for y in range(S):
        for x in range(S):
            r = rng.random()
            if r < 0.18:
                t[y][x] = (0, 0, 0, 0)
            else:
                t[y][x] = rng.choices([P["leaf0"], P["leaf1"], P["leaf2"]], [5, 3, 2])[0]
    return t

def cloudglass(rng):
    t = [[(*P["glass0"], 110)] * S for _ in range(S)]
    for i in range(S):
        t[0][i] = t[S - 1][i] = t[i][0] = t[i][S - 1] = (*P["glass1"], 220)
    for i in range(3, 7):  # a highlight streak
        t[i][i + 1] = (255, 255, 255, 200)
    return t

def skyiron(rng):
    t = driftstone(rng)
    for _ in range(4):
        x, y = rng.randrange(1, S - 2), rng.randrange(1, S - 2)
        for dx, dy in ((0, 0), (1, 0), (0, 1), (1, 1)):
            t[y + dy][x + dx] = P["iron0"] if (dx + dy) % 2 == 0 else P["iron1"]
    return t

def glowcap(rng):
    t = [[(0, 0, 0, 0)] * S for _ in range(S)]
    for y in range(9, S):
        for x in (7, 8):
            t[y][x] = P["stem"]
    for y in range(3, 9):
        w = (y - 2) + 1
        for x in range(8 - w, 8 + w):
            if 0 <= x < S:
                t[y][x] = P["cap1"] if (x + y) % 3 else P["cap0"]
    return t

def sailcloth(rng):
    t = [[P["sail0"] if (x + y) % 2 == 0 else P["sail1"] for x in range(S)] for y in range(S)]
    for x in range(S):  # a stitched hem
        if x % 2 == 0:
            t[2][x] = t[13][x] = P["bark1"]
    return t

def thatch(rng):
    t = [[P["straw0"]] * S for _ in range(S)]
    for y in range(S):
        for x in range(S):
            if (x + y // 2) % 3 == 0:
                t[y][x] = P["straw1"]
            elif rng.random() < 0.2:
                t[y][x] = P["straw2"]
    return t

def planks(rng):
    t = [[P["plank0"]] * S for _ in range(S)]
    for y in range(S):
        if y % 4 == 3:
            for x in range(S):
                t[y][x] = P["plank1"]
    for row in range(4):
        x = (row * 5 + 3) % S
        for y in range(row * 4, row * 4 + 3):
            t[y][x] = P["plank1"]
    return t

# Layer order is the source of truth for block faces; keep it in sync with
# `Layer` in src/blocks.ks.
TILES = [driftstone, loam, moss_top, moss_side, bark, rings, leaves, cloudglass,
         skyiron, glowcap, sailcloth, thatch, planks]

def main():
    img = Image.new("RGBA", (S, S * len(TILES)))
    for i, make in enumerate(TILES):
        rng = random.Random(1000 + i)
        tile = make(rng)
        for y in range(S):
            for x in range(S):
                c = tile[y][x]
                img.putpixel((x, i * S + y), c if len(c) == 4 else (*c, 255))
    img.save("assets/blocks.png")
    print(f"wrote assets/blocks.png ({len(TILES)} tiles)")

if __name__ == "__main__":
    main()
