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
from PIL import Image, ImageDraw, ImageFilter, ImageFont

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


def main():
    atlas = Image.open("assets/blocks.png").convert("RGBA")
    font = ImageFont.truetype(FONT_PATH, FONT_SIZE)
    layers = [
        ("Slot", rounded((255, 255, 255, 60), (255, 255, 255, 150), 3, 20, inset=3)),
        ("SlotSelected", rounded((255, 252, 240, 150), (255, 226, 150, 255), 5, 20, glow=(255, 214, 120, 255), inset=8)),
        ("Panel", rounded((30, 34, 62, 215), (255, 255, 255, 70), 2, 46)),
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
