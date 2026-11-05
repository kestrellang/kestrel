module Aerie

import linalg.(Vec4)

// The overlay: text, the hotbar, the title screen. Everything here is a
// list of `OverlayQuad`s, drawn over the world in one instanced draw.
// Sprites, block icons and glyphs come from the interface atlas
// (`assets/ui.png`, laid out in `ui.ks`).

/// A plain overlay rectangle.
func rect(x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> OverlayQuad {
    OverlayQuad(rect: Vec4(x: x, y: y, z: w, w: h), color: color, tile: Vec4(x: -1.0, y: 0.0, z: 0.0, w: 0.0))
}

/// Interface atlas layer `layer` drawn over (x, y, w, h), tinted by `color`.
func uiQuad(layer: Float32, x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> OverlayQuad {
    OverlayQuad(rect: Vec4(x: x, y: y, z: w, w: h), color: color, tile: Vec4(x: layer, y: 1.0, z: 0.0, w: 1.0))
}

/// As `uiQuad`, for pixel art: sampled crisp, on its 24-pixel grid.
func pixelQuad(layer: Float32, x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> OverlayQuad {
    OverlayQuad(rect: Vec4(x: x, y: y, z: w, w: h), color: color, tile: Vec4(x: layer, y: 2.0, z: 0.0, w: 1.0))
}

/// The overlay source for a sprite: crisp for pixel art, else smooth.
func source(sprite: UISprite) -> Float32 {
    if sprite.pixelArt { 2.0 } else { 1.0 }
}

func sprite(sprite: UISprite, x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> OverlayQuad {
    OverlayQuad(rect: Vec4(x: x, y: y, z: w, w: h), color: color, tile: Vec4(x: sprite.layer, y: source(sprite), z: 0.0, w: 1.0))
}

/// A sprite stretched to any width without stretching its rounded ends:
/// its left and right halves are drawn as end caps, `h / 2` wide, and its
/// middle column fills the rest.
func slicedSprite(sprite: UISprite, x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> [OverlayQuad] {
    let cap = h * 0.5;
    let layer = sprite.layer;
    let from = source(sprite);
    [
        OverlayQuad(rect: Vec4(x: x, y: y, z: cap, w: h), color: color, tile: Vec4(x: layer, y: from, z: 0.0, w: 0.5)),
        OverlayQuad(rect: Vec4(x: x + cap, y: y, z: w - cap * 2.0, w: h), color: color, tile: Vec4(x: layer, y: from, z: 0.5, w: 0.5)),
        OverlayQuad(rect: Vec4(x: x + w - cap, y: y, z: cap, w: h), color: color, tile: Vec4(x: layer, y: from, z: 0.5, w: 1.0))
    ]
}

let white: Vec4 = Vec4(x: 1.0, y: 1.0, z: 1.0, w: 1.0);
/// Deep slate blue, for text shadows and outlines.
let ink: Vec4 = Vec4(x: 0.13, y: 0.15, z: 0.30, w: 1.0);

/// `color` with its alpha scaled by `a`.
func faded(color: Vec4, a: Float32) -> Vec4 {
    Vec4(x: color.x, y: color.y, z: color.z, w: color.w * a)
}

/// The nearest text size, at least `size`'s floor, at which each pixel of
/// the font is a whole number of screen pixels: a multiple of the font's
/// 9 rows.
func crispSize(size: Float32) -> Float32 {
    ((size / 9.0).floor() * 9.0).clamp(9.0, 1000.0)
}

/// Width in pixels of `text` set at `size` pixels.
func textWidth(text: String, size: Float32) -> Float32 {
    var width: Float32 = 0.0;
    for byte in text.bytes {
        width = width + uiAdvance(byte);
    }
    width * size / uiFontSize
}

/// `text` set at `size` pixels (best a `crispSize`) with the top of its
/// tallest letters at y, one glyph quad per character.
func textQuads(text: String, x x: Float32, y y: Float32, size size: Float32, color color: Vec4) -> [OverlayQuad] {
    let k = size / uiFontSize;
    let cell = uiCell * k;
    var quads: [OverlayQuad] = [];
    // Whole pixels, so the font's pixels line up with the screen's.
    var pen = x.floor();
    let top = y.floor();
    for byte in text.bytes {
        if byte != 32 {
            // Each glyph sits `uiGlyphPad` in from its cell's left and 4
            // pixels down from its top (see tools/make_ui.py).
            quads.append(pixelQuad(uiGlyphLayer(byte), pen - uiGlyphPad * k, top - 4.0 * k, cell, cell, color));
        }
        pen = pen + uiAdvance(byte) * k;
    }
    quads
}

/// `text` over a drop shadow one font pixel down and right, so it reads
/// over a bright sky.
func labelQuads(text: String, x x: Float32, y y: Float32, size size: Float32, color color: Vec4) -> [OverlayQuad] {
    let offset = (size / 9.0).floor().clamp(1.0, 100.0);
    var quads = textQuads(text, x: x + offset, y: y + offset, size: size, color: faded(ink, color.w * 0.8));
    quads.append(contentsOf: textQuads(text, x: x, y: y, size: size, color: color));
    quads
}

/// `text` ringed all around by `rim`, for big text such as the title.
func outlinedQuads(text: String, x x: Float32, y y: Float32, size size: Float32, color color: Vec4, rim rim: Vec4) -> [OverlayQuad] {
    let r = (size * 0.045).clamp(1.0, 100.0);
    var quads: [OverlayQuad] = [];
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (-0.7, -0.7), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)] {
        quads.append(contentsOf: textQuads(text, x: x + dx * r, y: y + dy * r, size: size, color: rim));
    }
    quads.append(contentsOf: textQuads(text, x: x, y: y, size: size, color: color));
    quads
}

/// `text` centered on `width`, as `labelQuads`.
func centeredLabel(text: String, width width: Float32, y y: Float32, size size: Float32, color color: Vec4) -> [OverlayQuad] {
    labelQuads(text, x: ((width - textWidth(text, size)) * 0.5).floor(), y: y, size: size, color: color)
}

/// `text` centered on `width` on a wooden board, for lines that sit over
/// bright sky.
func pillLabel(text: String, width width: Float32, y y: Float32, size size: Float32, color color: Vec4) -> [OverlayQuad] {
    boardLabel(text, width: width, y: y, size: size, color: color, board: .Panel)
}

/// `text` centered on `width` on `board`, a sliced sprite, `roomy` times
/// the usual space around the text.
func boardLabel(text: String, width width: Float32, y y: Float32, size size: Float32, color color: Vec4, board board: UISprite, roomy roomy: Float32 = 1.0) -> [OverlayQuad] {
    let textW = textWidth(text, size);
    let height = (size * 2.0 * roomy).floor();
    let pad = (size * 1.3 * roomy).floor();
    var quads = slicedSprite(board, ((width - textW) * 0.5 - pad).floor(), (y - (height - size * 0.95) * 0.5).floor(), textW + pad * 2.0, height, white);
    quads.append(contentsOf: centeredLabel(text, width: width, y: y, size: size, color: color));
    quads
}

/// A message in the top left on a small frosted pill, fading out over
/// its last second.
func noticeQuads(text: String, fade fade: Float32) -> [OverlayQuad] {
    let size: Float32 = 18.0;
    let width = textWidth(text, size);
    var quads = slicedSprite(.Panel, 14.0, 14.0, width + 36.0, 40.0, faded(white, fade));
    quads.append(contentsOf: labelQuads(text, x: 32.0, y: 24.0, size: size, color: faded(white, fade)));
    quads
}

/// Frames per second, in the top left on a small board.
func fpsQuads(fps: Int64) -> [OverlayQuad] {
    let text = "\(fps) FPS";
    let size: Float32 = 18.0;
    var quads = slicedSprite(.Panel, 14.0, 14.0, textWidth(text, size) + 44.0, 40.0, white);
    quads.append(contentsOf: labelQuads(text, x: 36.0, y: 25.0, size: size, color: white));
    quads
}

/// The crosshair, the hotbar and the selected block's name for a `width`
/// by `height` screen, `time` seconds in.
func hud(width: Int64, height: Int64, selected: Int64, gliding: Bool, time: Float32) -> [OverlayQuad] {
    var quads: [OverlayQuad] = [];
    let w = Float32(from: width);
    let h = Float32(from: height);
    // Crosshair, on a soft outline; it widens while gliding.
    let arm: Float32 = if gliding { 13.0 } else { 8.0 };
    let (cx, cy) = ((w * 0.5).floor(), (h * 0.5).floor());
    let outline = faded(ink, 0.45);
    let cross = Vec4(x: 1.0, y: 1.0, z: 1.0, w: 0.9);
    quads.append(rect(cx - arm - 1.0, cy - 2.0, arm * 2.0 + 2.0, 4.0, outline));
    quads.append(rect(cx - 2.0, cy - arm - 1.0, 4.0, arm * 2.0 + 2.0, outline));
    quads.append(rect(cx - arm, cy - 1.0, arm * 2.0, 2.0, cross));
    quads.append(rect(cx - 1.0, cy - arm, 2.0, arm * 2.0, cross));

    // The hotbar: a wooden tray of stone slots, each with a little 3D
    // block. The selected slot turns gold and rises with a warm glow, its
    // block bobbing, and its name floats above. Slots are a whole multiple
    // of the sprites' 24 pixels, so the pixel art stays crisp.
    let slot: Float32 = 72.0;
    let gap: Float32 = 4.0;
    let count = Float32(from: hotbar.count);
    let total = count * slot + (count - 1.0) * gap;
    let left = ((w - total) * 0.5).floor();
    let top = h - slot - 22.0;
    quads.append(contentsOf: slicedSprite(.Panel, left - 22.0, top - 12.0, total + 44.0, slot + 24.0, white));
    var i: Int64 = 0;
    while i < hotbar.count {
        let chosen = i == selected;
        let x = left + Float32(from: i) * (slot + gap);
        let block = hotbar(i);
        if chosen {
            let lift: Float32 = 6.0;
            quads.append(sprite(.Glow, x - 30.0, top - lift - 30.0, slot + 60.0, slot + 60.0, Vec4(x: 1.0, y: 0.86, z: 0.55, w: 0.45)));
            quads.append(sprite(.SlotSelected, x, top - lift, slot, slot, white));
            let bob = ((time * 3.0).sin() * 2.0).floor();
            quads.append(uiQuad(uiIconLayer(block.id), x + 10.0, top - lift + 9.0 + bob, slot - 20.0, slot - 20.0, white));
        } else {
            quads.append(sprite(.Slot, x, top, slot, slot, white));
            quads.append(uiQuad(uiIconLayer(block.id), x + 14.0, top + 14.0, slot - 28.0, slot - 28.0, white));
        }
        i = i + 1;
    }
    quads.append(contentsOf: pillLabel(hotbar(selected).name, width: w, y: top - 54.0, size: 18.0, color: white));
    quads
}

/// The title screen, `time` seconds in: the name, bobbing in a soft glow
/// with twinkles around it, a pulsing prompt, and the controls.
func titleScreen(width: Int64, height: Int64, resumed: Bool, time: Float32) -> [OverlayQuad] {
    let w = Float32(from: width);
    let h = Float32(from: height);
    var quads: [OverlayQuad] = [];
    // The logo: the pixel font's letters, each pixel a driftstone block
    // capped with moss where open to the sky (see tools/make_ui.py).
    let columns = Float32(from: uiLogoColumns);
    let rows = Float32(from: uiLogoRows);
    let logoWidth = (w * 0.8).clamp(360.0, 1500.0).floor();
    let cell = (logoWidth / columns).floor();
    let size = cell * rows * 0.62;
    let titleWidth = cell * columns;
    let titleLeft = ((w - titleWidth) * 0.5).floor();
    let bob = (time * 1.3).sin() * cell * 0.05;
    let logoTop = (h * 0.12 + bob).floor();
    let titleTop = logoTop + cell * rows * 0.14;
    // A wide soft glow behind it, so it reads over any sky.
    quads.append(sprite(.Glow, w * 0.5 - titleWidth * 0.75, logoTop - cell * 0.6, titleWidth * 1.5, cell * rows + cell * 1.2, Vec4(x: 0.16, y: 0.18, z: 0.36, w: 0.4)));
    var r: Int64 = 0;
    while r < uiLogoRows {
        var c: Int64 = 0;
        while c < uiLogoColumns {
            let layer = uiLogoLayer + Float32(from: r * uiLogoColumns + c);
            quads.append(uiQuad(layer, titleLeft + Float32(from: c) * cell, logoTop + Float32(from: r) * cell, cell, cell, white));
            c = c + 1;
        }
        r = r + 1;
    }
    // Twinkles around the name, each on its own beat.
    let spots: [(Float32, Float32, Float32)] = [(0.08, 0.15, 0.0), (0.95, 0.2, 1.7), (0.25, -0.12, 3.1), (0.78, -0.1, 4.4), (0.5, 1.05, 2.3), (0.1, 0.9, 5.2), (0.92, 0.88, 0.9)];
    for (fx, fy, phase) in spots {
        let beat = (time * 2.2 + phase).sin() * 0.5 + 0.5;
        let s = size * (0.18 + beat * 0.22);
        let x = titleLeft + titleWidth * fx;
        let y = titleTop + size * 0.8 * fy;
        quads.append(sprite(.Twinkle, x - s * 0.5, y - s * 0.5, s, s, Vec4(x: 1.0, y: 0.97, z: 0.88, w: 0.4 + beat * 0.6)));
    }
    let small = crispSize((size * 0.2).clamp(18.0, 30.0));
    quads.append(contentsOf: pillLabel("Islands above the cloud sea", width: w, y: (logoTop + cell * rows + small * 0.6).floor(), size: small, color: Vec4(x: 0.92, y: 0.95, z: 1.0, w: 1.0)));
    // Bound first: an `if` directly inside an array literal crashes the
    // compiler (array_literal_if_expression_owned_element.ks).
    let prompt = if resumed { "Press Enter or click to continue" } else { "Press Enter or click to begin" };
    // The prompt on a warm wooden button, with a glow that breathes.
    let pulse = 0.5 + 0.5 * (time * 3.0).sin();
    // Big: it's the one thing to press.
    let promptSize = crispSize(small * 1.7);
    let promptY = (h * 0.68).floor();
    let promptW = textWidth(prompt, promptSize) + promptSize * 3.4;
    quads.append(sprite(.Glow, w * 0.5 - promptW * 0.7, promptY - promptSize * 2.2, promptW * 1.4, promptSize * 5.0, Vec4(x: 1.0, y: 0.82, z: 0.5, w: 0.15 + pulse * 0.2)));
    quads.append(contentsOf: boardLabel(prompt, width: w, y: promptY, size: promptSize, color: white, board: .Button, roomy: 1.3));
    quads
}
