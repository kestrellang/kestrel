"""Draws Aerie's interface atlas, assets/ui.png, and writes src/ui.ks, the
layer numbers and glyph widths the game needs to use it.

The atlas is a column of 96x96 layers, loaded as one texture array:

- sprites: a hotbar slot, a selected slot, a rounded panel (drawn in three
  slices so it can stretch), a soft glow and a twinkle;
- an icon per block: a little 3D cube built from the block atlas's faces
  (run tools/make_atlas.py first), or the sprite itself for plants;
- a glyph per printable ASCII character, from Inter SemiBold (SIL Open
  Font License), white with coverage as alpha, so text renders smooth at
  any size and in any color.

Needs Inter installed (Debian/Ubuntu: `apt install fonts-inter`).

Run: python3 tools/make_ui.py   (from examples/aerie)
"""
import random

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

CELL = 96
FONT_PATH = "/usr/share/fonts/opentype/inter/Inter-SemiBold.otf"
FONT_SIZE = 72
SUPER = 4  # supersampling for smooth edges

# Block atlas layers (see `Layer` in src/blocks.ks).
DRIFTSTONE, LOAM, MOSS_TOP, MOSS_SIDE, BARK, RINGS, EMBERLEAF, CLOUDGLASS, \
    SKYIRON, GLOWCAP, SAILCLOTH, FEATHERTHATCH, PLANKS, ASHROCK, SCORCH_TOP, \
    SCORCH_SIDE, DARKBRICK, EMBERCORE, DEADWOOD, DEAD_RINGS, GLOOMLEAF, HEXBLOOM = range(22)

# Icons in block id order (id 1 first): (top, side) layers, or one layer
# for a plant drawn flat.
ICONS = [
    ("Driftstone", (DRIFTSTONE, DRIFTSTONE)),
    ("Loam", (LOAM, LOAM)),
    ("Mossloam", (MOSS_TOP, MOSS_SIDE)),
    ("Emberwood", (RINGS, BARK)),
    ("Emberleaf", (EMBERLEAF, EMBERLEAF)),
    ("Cloudglass", (CLOUDGLASS, CLOUDGLASS)),
    ("Skyiron", (SKYIRON, SKYIRON)),
    ("Glowcap", GLOWCAP),
    ("Sailcloth", (SAILCLOTH, SAILCLOTH)),
    ("Featherthatch", (FEATHERTHATCH, FEATHERTHATCH)),
    ("Planks", (PLANKS, PLANKS)),
    ("Ashrock", (ASHROCK, ASHROCK)),
    ("Scorchloam", (SCORCH_TOP, SCORCH_SIDE)),
    ("Darkbrick", (DARKBRICK, DARKBRICK)),
    ("Embercore", (EMBERCORE, EMBERCORE)),
    ("Deadwood", (DEAD_RINGS, DEADWOOD)),
    ("Gloomleaf", (GLOOMLEAF, GLOOMLEAF)),
    ("Hexbloom", HEXBLOOM),
]


def rounded(fill, outline, width, radius, glow=None, inset=0):
    """A rounded square sprite, drawn large and scaled down."""
    big = CELL * SUPER
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    pad = inset * SUPER
    box = (pad, pad, big - 1 - pad, big - 1 - pad)
    if glow:
        halo = Image.new("RGBA", (big, big), (0, 0, 0, 0))
        ImageDraw.Draw(halo).rounded_rectangle(box, radius * SUPER, outline=glow, width=width * SUPER * 2)
        img = Image.alpha_composite(img, halo.filter(ImageFilter.GaussianBlur(5 * SUPER)))
        d = ImageDraw.Draw(img)
    d.rounded_rectangle(box, radius * SUPER, fill=fill, outline=outline, width=width * SUPER)
    # A soft highlight across the top half.
    top = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    inner = (box[0] + width * SUPER, box[1] + width * SUPER, box[2] - width * SUPER, (box[1] + box[3]) // 2)
    ImageDraw.Draw(top).rounded_rectangle(inner, (radius - width) * SUPER, fill=(255, 255, 255, 34))
    img = Image.alpha_composite(img, top)
    return img.resize((CELL, CELL), Image.LANCZOS)


# Pixel-art pieces are drawn on a 24x24 grid and scaled up 4x, so they
# match the blocks' chunky pixels.
GRID = 24
INK = (30, 22, 34, 255)
# Dusky walnut for panels, a warmer wood for buttons, slate stone for
# slots and gold for the selected one.
DARK_WOOD = [(54, 38, 52), (64, 46, 60), (76, 54, 68), (90, 64, 80)]
WARM_WOOD = [(128, 78, 70), (148, 94, 80), (168, 112, 92), (188, 132, 106)]
STONE_LIGHT, STONE_MID, STONE_DARK = (206, 202, 228), (150, 144, 182), (96, 90, 128)
GOLD_LIGHT, GOLD_MID, GOLD_DARK = (255, 238, 170), (240, 196, 100), (176, 120, 56)


def pixels(draw):
    """A GRID x GRID pixel-art sprite from `draw(put)`, scaled up crisp."""
    img = Image.new("RGBA", (GRID, GRID), (0, 0, 0, 0))
    def put(x, y, c):
        if 0 <= x < GRID and 0 <= y < GRID:
            img.putpixel((x, y), c if len(c) == 4 else (*c, 255))
    draw(put)
    return img.resize((CELL, CELL), Image.NEAREST)


def chunky_box(x0, y0, x1, y1, cut):
    """Whether (x, y) is inside a box with its corners cut in `cut` steps."""
    def inside(x, y):
        if not (x0 <= x <= x1 and y0 <= y <= y1):
            return False
        dx = min(x - x0, x1 - x)
        dy = min(y - y0, y1 - y)
        return dx + dy >= cut
    return inside


def plank_pill(wood, rim_light, rim_dark, trim, top, bottom):
    """A wooden board with rounded pixel ends and rivets at each end. The
    grain runs across, so the middle column can stretch to any width
    (see `slicedSprite`)."""
    rng = random.Random(5)
    grain = [rng.random() for _ in range(GRID)]
    def draw(put):
        outer = chunky_box(0, top, GRID - 1, bottom, 4)
        inner = chunky_box(1, top + 1, GRID - 2, bottom - 1, 4)
        for y in range(GRID):
            for x in range(GRID):
                if not outer(x, y):
                    continue
                if not inner(x, y):
                    put(x, y, INK)
                elif not inner(x, y - 1) or not inner(x - 1, y) and y < (top + bottom) // 2:
                    put(x, y, rim_light)
                elif not inner(x, y + 1) or not inner(x + 1, y) and y > (top + bottom) // 2:
                    put(x, y, rim_dark)
                elif y == (top + bottom) // 2:
                    put(x, y, wood[0])  # the seam between two boards
                else:
                    put(x, y, wood[min(3, int(grain[y] * 3.2) + (1 if y < (top + bottom) // 2 else 0))])
        mid = (top + bottom) // 2
        for x in (4, GRID - 5):
            for y in (mid - 3, mid + 3):
                put(x, y, trim[0])
                put(x + 1, y, trim[1])
                put(x, y + 1, trim[1])
                put(x + 1, y + 1, trim[2])
    return pixels(draw)


def slot(frame_light, frame_dark, recess, gems=None):
    """A hotbar slot: a beveled frame around a recessed hollow."""
    def draw(put):
        outer = chunky_box(0, 0, GRID - 1, GRID - 1, 2)
        frame = chunky_box(1, 1, GRID - 2, GRID - 2, 2)
        hollow = chunky_box(4, 4, GRID - 5, GRID - 5, 1)
        for y in range(GRID):
            for x in range(GRID):
                if not outer(x, y):
                    continue
                if not frame(x, y):
                    put(x, y, INK)
                elif hollow(x, y):
                    # A shadow along the hollow's top and left: it's sunk in.
                    shade = 0.7 if (x == 4 or y == 4) else 1.0
                    put(x, y, (int(recess[0] * shade), int(recess[1] * shade), int(recess[2] * shade), recess[3]))
                elif x + y < GRID - 1:
                    put(x, y, frame_light)
                else:
                    put(x, y, frame_dark)
        # A ring between frame and hollow.
        for i in range(4, GRID - 4):
            for x, y in ((i, 3), (3, i), (i, GRID - 4), (GRID - 4, i)):
                if outer(x, y) and not hollow(x, y):
                    put(x, y, INK)
        if gems:
            for x, y in ((2, 2), (GRID - 3, 2), (2, GRID - 3), (GRID - 3, GRID - 3)):
                put(x, y, gems)
    return pixels(draw)


def logo(atlas, text, columns, rows):
    """The title, built from the game's own blocks: driftstone letters
    capped with moss, extruded downward into darker stone and outlined.
    Returns `columns` x `rows` cells, left to right, top to bottom."""
    w, h = CELL * columns, CELL * rows
    font = ImageFont.truetype("/usr/share/fonts/opentype/inter/InterDisplay-Black.otf", int(h * 0.8))
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).text((w / 2, h * 0.45), text, font=font, fill=255, anchor="mm")
    mask = mask.point(lambda v: 255 if v > 110 else 0)

    def tiled(layer, scale):
        tile = atlas.crop((0, layer * 16, 16, layer * 16 + 16)).resize((16 * scale, 16 * scale), Image.NEAREST)
        out = Image.new("RGBA", (w, h))
        for y in range(0, h, tile.height):
            for x in range(0, w, tile.width):
                out.paste(tile, (x, y))
        return out

    stone = tiled(DRIFTSTONE, 4)
    moss = tiled(MOSS_TOP, 4)
    dark = Image.eval(stone, lambda v: int(v * 0.5)).convert("RGB").convert("RGBA")
    # The extrusion: the letters stacked downward in shaded stone.
    depth = 14
    body = Image.new("L", (w, h), 0)
    for d in range(1, depth + 1):
        body.paste(255, (0, 0), ImageChops.offset(mask, d // 3, d))
    body = ImageChops.lighter(body, mask)
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    # The outline: everything grown by a few pixels, in ink.
    ring = body.filter(ImageFilter.MaxFilter(9))
    out.paste(Image.new("RGBA", (w, h), INK), (0, 0), ring)
    out.paste(dark, (0, 0), body)
    out.paste(stone, (0, 0), mask)
    # Moss on top: the band of each letter within a few pixels of open
    # air above, ragged by column so it drips.
    rng = random.Random(9)
    drips = [rng.choice([14, 16, 18, 20, 24, 28]) for _ in range(w // 4 + 1)]
    m = mask.load()
    grass = Image.new("L", (w, h), 0)
    g = grass.load()
    for x in range(w):
        run = 0
        for y in range(h):
            if m[x, y]:
                run += 1
                if run <= drips[x // 4]:
                    g[x, y] = 255
            else:
                run = 0
    out.paste(moss, (0, 0), grass)
    # A light lip along the very top of the moss.
    lip = ImageChops.subtract(grass, ImageChops.offset(grass, 0, 4))
    out.paste(Image.new("RGBA", (w, h), (226, 252, 230, 255)), (0, 0), lip)
    return [out.crop((c * CELL, r * CELL, (c + 1) * CELL, (r + 1) * CELL)) for r in range(rows) for c in range(columns)]


def glow():
    img = Image.new("RGBA", (CELL, CELL), (0, 0, 0, 0))
    c = (CELL - 1) / 2
    for y in range(CELL):
        for x in range(CELL):
            d = ((x - c) ** 2 + (y - c) ** 2) ** 0.5 / c
            a = max(0.0, 1.0 - d) ** 2
            img.putpixel((x, y), (255, 255, 255, int(a * 255)))
    return img


def twinkle():
    """A four-pointed star with a soft core."""
    img = Image.new("RGBA", (CELL, CELL), (0, 0, 0, 0))
    c = (CELL - 1) / 2
    for y in range(CELL):
        for x in range(CELL):
            dx, dy = abs(x - c) / c, abs(y - c) / c
            # Thin arms that taper: inside when one axis is small enough.
            arm = max(0.0, 1.0 - (dx * dy) ** 0.5 * 9.0 - max(dx, dy) * 0.9)
            core = max(0.0, 1.0 - (dx * dx + dy * dy) ** 0.5 * 2.2)
            a = min(1.0, arm + core)
            img.putpixel((x, y), (255, 255, 255, int(a * 255)))
    return img


def face_texel(atlas, layer, s, t):
    """The block atlas texel at (s, t), each 0 to 1, of `layer`."""
    x = min(15, max(0, int(s * 16)))
    y = min(15, max(0, int(t * 16)))
    return atlas.getpixel((x, layer * 16 + y))


def cube_icon(atlas, top, side):
    """An isometric cube: the top face, and the two front faces shaded
    darker, drawn supersampled for clean edges."""
    big = CELL * SUPER
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    cx = big / 2
    a, b, h = 40 * SUPER, 20 * SUPER, 44 * SUPER
    y0 = 5 * SUPER
    px = img.load()
    for y in range(big):
        for x in range(big):
            fx = (x + 0.5 - cx) / a
            fy = y + 0.5 - y0
            # Top face (w = 1): u - v = fx, u + v = fy / b.
            u = (fy / b + fx) / 2
            v = (fy / b - fx) / 2
            if 0 <= u < 1 and 0 <= v < 1:
                c = face_texel(atlas, top, u, v)
                px[x, y] = (c[0], c[1], c[2], c[3])
                continue
            # Left front face (v = 1).
            u = fx + 1
            if 0 <= u < 1:
                w = 1 - (fy - (u + 1) * b) / h
                if 0 <= w < 1:
                    c = face_texel(atlas, side, u, 1 - w)
                    px[x, y] = (int(c[0] * 0.80), int(c[1] * 0.80), int(c[2] * 0.84), c[3])
                    continue
            # Right front face (u = 1).
            v = 1 - fx
            if 0 <= v < 1:
                w = 1 - (fy - (1 + v) * b) / h
                if 0 <= w < 1:
                    c = face_texel(atlas, side, 1 - v, 1 - w)
                    px[x, y] = (int(c[0] * 0.62), int(c[1] * 0.62), int(c[2] * 0.70), c[3])
    return img.resize((CELL, CELL), Image.LANCZOS)


def sprite_icon(atlas, layer):
    tile = atlas.crop((0, layer * 16, 16, layer * 16 + 16))
    return tile.resize((80, 80), Image.NEAREST)


def glyph(font, ch):
    img = Image.new("L", (CELL, CELL), 0)
    ascent, _ = font.getmetrics()
    ImageDraw.Draw(img).text((GLYPH_PAD, 4 + ascent), ch, font=font, fill=255, anchor="ls")
    out = Image.new("RGBA", (CELL, CELL), (255, 255, 255, 0))
    out.putalpha(img)
    return out


GLYPH_PAD = 12
LOGO_COLUMNS, LOGO_ROWS = 6, 2


def main():
    atlas = Image.open("assets/blocks.png").convert("RGBA")
    font = ImageFont.truetype(FONT_PATH, FONT_SIZE)
    layers = [
        ("Slot", slot(STONE_MID, STONE_DARK, (40, 34, 58, 200))),
        ("SlotSelected", slot(GOLD_LIGHT, GOLD_DARK, (84, 66, 96, 190), gems=(150, 240, 230))),
        ("Panel", plank_pill(DARK_WOOD, (124, 96, 112), (40, 28, 40), (GOLD_LIGHT, GOLD_MID, GOLD_DARK), 2, 21)),
        ("Button", plank_pill(WARM_WOOD, (226, 176, 140), (96, 56, 52), (GOLD_LIGHT, GOLD_MID, GOLD_DARK), 1, 22)),
        ("Glow", glow()),
        ("Twinkle", twinkle()),
    ]
    for name, faces in ICONS:
        if isinstance(faces, tuple):
            icon = cube_icon(atlas, *faces)
        else:
            icon = Image.new("RGBA", (CELL, CELL), (0, 0, 0, 0))
            icon.paste(sprite_icon(atlas, faces), (8, 8))
        layers.append(("Icon" + name, icon))
    logo_base = len(layers)
    for cell in logo(atlas, "Aerie", LOGO_COLUMNS, LOGO_ROWS):
        layers.append((None, cell))
    glyph_base = len(layers)
    advances = []
    for code in range(32, 127):
        layers.append((None, glyph(font, chr(code))))
        advances.append(font.getlength(chr(code)))

    sheet = Image.new("RGBA", (CELL, CELL * len(layers)), (0, 0, 0, 0))
    for i, (_, img) in enumerate(layers):
        sheet.paste(img, (0, i * CELL))
    sheet.save("assets/ui.png")

    named = [(n, i) for i, (n, _) in enumerate(layers) if n and not n.startswith("Icon")]
    lines = [
        "module Aerie",
        "",
        "// The interface atlas's layout: generated by tools/make_ui.py with",
        "// assets/ui.png; edit the generator, not this file.",
        "",
        "/// Each layer is this many pixels square.",
        f"let uiCellSize: Int64 = {CELL};",
        f"let uiCell: Float32 = {CELL}.0;",
        "/// The font's pixel size inside a glyph layer, and where its glyphs start.",
        f"let uiFontSize: Float32 = {FONT_SIZE}.0;",
        f"let uiGlyphPad: Float32 = {GLYPH_PAD}.0;",
        "",
        "/// The title logo: this many cells across and down, from this layer,",
        "/// left to right then top to bottom.",
        f"let uiLogoLayer: Float32 = {logo_base}.0;",
        f"let uiLogoColumns: Int64 = {LOGO_COLUMNS};",
        f"let uiLogoRows: Int64 = {LOGO_ROWS};",
        "",
        "/// A sprite in the interface atlas.",
        "enum UISprite {",
    ]
    lines += [f"    case {n}" for n, _ in named]
    lines += [
        "",
        "    var layer: Float32 {",
        "        match self {",
        "            " + ", ".join(f".{n} => {i}.0" for n, i in named),
        "        }",
        "    }",
        "}",
        "",
        "/// The icon layer for block id `id` (1 and up).",
        "func uiIconLayer(id: UInt16) -> Float32 {",
        f"    Float32(from: Int64(from: id) - 1 + {len(named)})",
        "}",
        "",
        "/// The glyph layer for an ASCII byte; anything unprintable shows as `?`.",
        "func uiGlyphLayer(byte: UInt8) -> Float32 {",
        "    let code = Int64(from: byte);",
        f"    let printable = if code >= 32 and code <= 126 {{ code }} else {{ 63 }};",
        f"    Float32(from: printable - 32 + {glyph_base})",
        "}",
        "",
        "/// How far the pen moves after a byte, in font pixels at `uiFontSize`.",
        "func uiAdvance(byte: UInt8) -> Float32 {",
        "    match Int64(from: byte) {",
    ]
    for code, adv in zip(range(32, 127), advances):
        lines.append(f"        {code} => {adv:.2f},")
    lines += [
        f"        _ => {font.getlength('?'):.2f}",
        "    }",
        "}",
        "",
    ]
    with open("src/ui.ks", "w") as f:
        f.write("\n".join(lines))
    print(f"wrote assets/ui.png ({len(layers)} layers) and src/ui.ks")


if __name__ == "__main__":
    main()
