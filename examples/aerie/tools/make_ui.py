"""Draws Aerie's interface atlas, assets/ui.png, and writes src/ui/ui.ks, the
layer numbers and glyph widths the game needs to use it.

The atlas is a column of 96x96 layers, loaded as one texture array:

- sprites: a hotbar slot, a selected slot, a wooden panel and button (drawn in three
  slices so it can stretch), a soft glow and a twinkle;
- the title logo, the pixel font's letters built from block textures
  (run tools/make_atlas.py first);
- a glyph per printable ASCII character from our own pixel font
  (tools/pixel_font.py), white, so text can be drawn in any color.

Run: python3 tools/make_ui.py   (from examples/aerie)
"""
import random

from PIL import Image, ImageDraw, ImageFilter

import pixel_font

CELL = 96
# The font's "size": its full height, ascender to descender, in cell pixels.
FONT_SIZE = pixel_font.ROWS * 4
SUPER = 4  # supersampling for smooth edges

# Block atlas layers (see `Layer` in src/world/blocks.ks).
DRIFTSTONE, LOAM, MOSS_TOP, MOSS_SIDE, BARK, RINGS, EMBERLEAF, CLOUDGLASS, \
    SKYIRON, GLOWCAP, SAILCLOTH, FEATHERTHATCH, PLANKS, ASHROCK, SCORCH_TOP, \
    SCORCH_SIDE, DARKBRICK, EMBERCORE, DEADWOOD, DEAD_RINGS, GLOOMLEAF, HEXBLOOM = range(22)

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
    """The title in the pixel font, each of its pixels a little block:
    driftstone, with moss on any block open to the sky, set over a darker
    stack of the same blocks for depth and outlined in ink. Returns
    `columns` x `rows` cells, left to right, top to bottom."""
    w, h = CELL * columns, CELL * rows
    # Each font pixel is one block tile, at its own 16 pixels.
    unit = 16
    stone = atlas.crop((0, DRIFTSTONE * 16, 16, DRIFTSTONE * 16 + 16)).resize((unit, unit), Image.NEAREST)
    moss = atlas.crop((0, MOSS_TOP * 16, 16, MOSS_TOP * 16 + 16)).resize((unit, unit), Image.NEAREST)
    # The pixels of the text, as (column, row) cells.
    cells = set()
    pen = 0
    for ch in text:
        for y, row in enumerate(pixel_font.glyph(ch)):
            for x, c in enumerate(row):
                if c == "#":
                    cells.add((pen + x, y))
        pen += pixel_font.width(ch) + 1
    span = (pen - 1) * unit
    depth = (unit // 4, unit // 3)
    left = (w - span - depth[0]) // 2
    top = (h - 7 * unit - depth[1]) // 2
    rng = random.Random(9)
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    shadow = Image.eval(stone, lambda v: int(v * 0.45)).convert("RGB").convert("RGBA")
    for (cx, cy) in sorted(cells):
        out.paste(shadow, (left + cx * unit + depth[0], top + cy * unit + depth[1]))
    for (cx, cy) in sorted(cells):
        x, y = left + cx * unit, top + cy * unit
        out.paste(stone, (x, y))
        if (cx, cy - 1) not in cells:
            # Grass on top, a few pixels deep, ragged where it drips.
            cap = moss.crop((0, 0, unit, 5))
            out.paste(cap, (x, y))
            for dx in range(unit):
                if rng.random() < 0.3:
                    drip = rng.choice([1, 2, 3])
                    out.paste(moss.crop((dx, 5, dx + 1, 5 + drip)), (x + dx, y + 5))
            ImageDraw.Draw(out).rectangle((x, y, x + unit - 1, y), fill=(226, 252, 230, 255))
    ring = out.getchannel("A").filter(ImageFilter.MaxFilter(5))
    outlined = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    outlined.paste(Image.new("RGBA", (w, h), INK), (0, 0), ring)
    outlined = Image.alpha_composite(outlined, out)
    return [outlined.crop((c * CELL, r * CELL, (c + 1) * CELL, (r + 1) * CELL)) for r in range(rows) for c in range(columns)]


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


def glyph(ch):
    """A pixel-font glyph, white, each font pixel FONT_UNIT cell pixels,
    its top-left GLYPH_PAD in and 4 down."""
    img = Image.new("RGBA", (CELL, CELL), (255, 255, 255, 0))
    d = ImageDraw.Draw(img)
    for y, row in enumerate(pixel_font.glyph(ch)):
        for x, c in enumerate(row):
            if c == "#":
                x0, y0 = GLYPH_PAD + x * FONT_UNIT, GLYPH_PAD + y * FONT_UNIT
                d.rectangle((x0, y0, x0 + FONT_UNIT - 1, y0 + FONT_UNIT - 1), fill=(255, 255, 255, 255))
    return img


# The pixel font (tools/pixel_font.py): each font pixel is this many cell
# pixels, 24 to a cell like the other pixel art, so the glyphs snap to the
# same grid.
FONT_UNIT = 4
GLYPH_PAD = 4
LOGO_COLUMNS, LOGO_ROWS = 9, 2
PIXEL_ART = {"Slot", "SlotSelected", "Panel", "Button"}


def main():
    atlas = Image.open("assets/blocks.png").convert("RGBA")
    layers = [
        ("Slot", slot(STONE_MID, STONE_DARK, (40, 34, 58, 200))),
        ("SlotSelected", slot(GOLD_LIGHT, GOLD_DARK, (84, 66, 96, 190), gems=(150, 240, 230))),
        ("Panel", plank_pill(DARK_WOOD, (124, 96, 112), (40, 28, 40), (GOLD_LIGHT, GOLD_MID, GOLD_DARK), 2, 21)),
        ("Button", plank_pill(WARM_WOOD, (226, 176, 140), (96, 56, 52), (GOLD_LIGHT, GOLD_MID, GOLD_DARK), 1, 22)),
        ("Glow", glow()),
        ("Twinkle", twinkle()),
    ]
    logo_base = len(layers)
    for cell in logo(atlas, "Kestrelcraft", LOGO_COLUMNS, LOGO_ROWS):
        layers.append((None, cell))
    glyph_base = len(layers)
    advances = []
    for code in range(32, 127):
        layers.append((None, glyph(chr(code))))
        advances.append((pixel_font.width(chr(code)) + 1) * FONT_UNIT)

    sheet = Image.new("RGBA", (CELL, CELL * len(layers)), (0, 0, 0, 0))
    for i, (_, img) in enumerate(layers):
        sheet.paste(img, (0, i * CELL))
    sheet.save("assets/ui.png")

    named = [(n, i) for i, (n, _) in enumerate(layers) if n]
    lines = [
        "module Aerie",
        "",
        "// The interface atlas's layout: generated by tools/make_ui.py with",
        "// assets/ui.png; edit the generator, not this file.",
        "",
        "/// Each layer is this many pixels square.",
        f"let uiCellSize: Int64 = {CELL};",
        f"let uiCell: Float32 = {CELL}.0;",
        "/// The font's pixel size inside a glyph layer, and how far in from the",
        "/// layer's top left corner its glyphs start.",
        f"let uiFontSize: Float32 = {FONT_SIZE}.0;",
        f"let uiGlyphPad: Float32 = {GLYPH_PAD}.0;",
        "/// Rows of font pixels in a line, ascenders to descenders.",
        f"let uiFontRows: Float32 = {pixel_font.ROWS}.0;",
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
        "",
        "    /// Drawn on the 24-pixel grid, so sampled crisp (see `OverlayQuad.pixel`).",
        "    var pixelArt: Bool {",
        "        match self {",
        "            " + ", ".join(f".{n} => {'true' if n in PIXEL_ART else 'false'}" for n, _ in named),
        "        }",
        "    }",
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
        f"        _ => {(pixel_font.width('?') + 1) * FONT_UNIT:.2f}",
        "    }",
        "}",
        "",
    ]
    with open("src/ui/ui.ks", "w") as f:
        f.write("\n".join(lines))
    print(f"wrote assets/ui.png ({len(layers)} layers) and src/ui/ui.ks")


if __name__ == "__main__":
    main()
