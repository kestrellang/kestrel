module Aerie

import linalg.(Vec4)

/// A line of text in our pixel font, at `size` pixels (best a
/// `Label.crisp` size) in `color`.
struct Label {
    var text: String
    var size: Float32
    var color: Vec4

    /// The nearest text size, at least `size`'s floor, at which each pixel
    /// of the font is a whole number of screen pixels: a multiple of the
    /// font's rows.
    static func crisp(size: Float32) -> Float32 {
        ((size / uiFontRows).floor() * uiFontRows).clamp(uiFontRows, 1000.0)
    }

    /// How big one of the font's pixels is, in whole screen pixels.
    var fontPixel: Float32 {
        (self.size / uiFontRows).floor().clamp(1.0, 100.0)
    }

    /// Width in pixels.
    var width: Float32 {
        var width: Float32 = 0.0;
        for byte in self.text.bytes {
            width = width + uiAdvance(byte);
        }
        width * self.size / uiFontSize
    }

    /// One glyph quad per character, with the top of the tallest letters
    /// at y.
    func quads(x: Float32, y: Float32) -> [OverlayQuad] {
        let k = self.size / uiFontSize;
        let cell = uiCell * k;
        var quads: [OverlayQuad] = [];
        // Whole pixels, so the font's pixels line up with the screen's.
        var pen = x.floor();
        let top = y.floor();
        for byte in self.text.bytes {
            // Each glyph sits `uiGlyphPad` in from its cell's top left.
            if byte != 32 {
                quads.append(OverlayQuad.pixel(uiGlyphLayer(byte), pen - uiGlyphPad * k, top - uiGlyphPad * k, cell, cell, self.color));
            }
            pen = pen + uiAdvance(byte) * k;
        }
        quads
    }

    /// Over a drop shadow one font pixel down and right, so it reads over
    /// a bright sky.
    func shadowed(x: Float32, y: Float32) -> [OverlayQuad] {
        let offset = self.fontPixel;
        let shadow = Label(text: self.text, size: self.size, color: faded(ink, self.color.w * 0.8));
        var quads = shadow.quads(x + offset, y + offset);
        quads.append(contentsOf: self.quads(x, y));
        quads
    }

    /// Shadowed and centered across a `width`-pixel screen.
    func centered(width: Float32, y: Float32) -> [OverlayQuad] {
        self.shadowed(((width - self.width) * 0.5).floor(), y)
    }

    /// Centered on `board`, a sliced sprite, with `roomy` times the usual
    /// space around the text.
    func onBoard(board: UISprite, width width: Float32, y y: Float32, roomy roomy: Float32) -> [OverlayQuad] {
        let height = (self.size * 2.0 * roomy).floor();
        let pad = (self.size * 1.3 * roomy).floor();
        let left = ((width - self.width) * 0.5 - pad).floor();
        let top = (y - (height - self.size * 0.95) * 0.5).floor();
        var quads = OverlayQuad.sliced(board, left, top, self.width + pad * 2.0, height, white);
        quads.append(contentsOf: self.centered(width, y));
        quads
    }
}

/// Text for small labels, such as the corner ones.
let smallText: Float32 = 18.0;

/// `text` in the top left on a small board, `row` boards down (0 is the
/// top), faded by `alpha`.
func cornerLabel(text: String, row row: Int64, alpha alpha: Float32) -> [OverlayQuad] {
    let margin: Float32 = 14.0;
    let height: Float32 = 40.0;
    let pad: Float32 = 20.0;
    let y = margin + Float32(from: row) * (height + 6.0);
    let label = Label(text: text, size: smallText, color: faded(white, alpha));
    var quads = OverlayQuad.sliced(.Panel, margin, y, label.width + pad * 2.0, height, label.color);
    quads.append(contentsOf: label.shadowed(margin + pad, y + (height - smallText) * 0.5));
    quads
}
