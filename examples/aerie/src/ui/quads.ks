module Aerie

import linalg.(Vec4)

// The overlay is a list of `OverlayQuad`s, drawn over the world in one
// instanced draw. Sprites, block icons and glyphs come from the interface
// atlas (`assets/ui.png`, laid out in `ui.ks`).

let white: Vec4 = Vec4(x: 1.0, y: 1.0, z: 1.0, w: 1.0);
/// Deep slate blue, for text shadows and outlines.
let ink: Vec4 = Vec4(x: 0.13, y: 0.15, z: 0.30, w: 1.0);

/// `color` with its alpha scaled by `a`.
func faded(color: Vec4, a: Float32) -> Vec4 {
    Vec4(x: color.x, y: color.y, z: color.z, w: color.w * a)
}

extend OverlayQuad {
    /// A plain rectangle of `color` at (x, y), `w` by `h` pixels.
    static func fill(x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> OverlayQuad {
        OverlayQuad(rect: Vec4(x: x, y: y, z: w, w: h), color: color, tile: Vec4(x: -1.0, y: 0.0, z: 0.0, w: 0.0))
    }

    /// Interface atlas layer `layer` over (x, y, w, h), tinted by `color`,
    /// sampled smooth.
    static func ui(layer: Float32, x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> OverlayQuad {
        OverlayQuad(rect: Vec4(x: x, y: y, z: w, w: h), color: color, tile: Vec4(x: layer, y: 1.0, z: 0.0, w: 1.0))
    }

    /// As `ui`, for pixel art: sampled crisp, on its 24-pixel grid.
    static func pixel(layer: Float32, x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> OverlayQuad {
        OverlayQuad(rect: Vec4(x: x, y: y, z: w, w: h), color: color, tile: Vec4(x: layer, y: 2.0, z: 0.0, w: 1.0))
    }

    /// `sprite` over (x, y, w, h), crisp if it's pixel art.
    static func sprite(sprite: UISprite, x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> OverlayQuad {
        OverlayQuad(rect: Vec4(x: x, y: y, z: w, w: h), color: color, tile: Vec4(x: sprite.layer, y: sprite.source, z: 0.0, w: 1.0))
    }

    /// `sprite` stretched to any width without stretching its rounded
    /// ends: its left and right halves are drawn as end caps, `h / 2` wide,
    /// and its middle column fills the rest.
    static func sliced(sprite: UISprite, x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> [OverlayQuad] {
        let cap = h * 0.5;
        let (layer, from) = (sprite.layer, sprite.source);
        [
            OverlayQuad(rect: Vec4(x: x, y: y, z: cap, w: h), color: color, tile: Vec4(x: layer, y: from, z: 0.0, w: 0.5)),
            OverlayQuad(rect: Vec4(x: x + cap, y: y, z: w - cap * 2.0, w: h), color: color, tile: Vec4(x: layer, y: from, z: 0.5, w: 0.5)),
            OverlayQuad(rect: Vec4(x: x + w - cap, y: y, z: cap, w: h), color: color, tile: Vec4(x: layer, y: from, z: 0.5, w: 1.0))
        ]
    }
}

extend UISprite {
    /// The overlay source for this sprite: crisp (2) for pixel art, else
    /// smooth (1).
    var source: Float32 {
        if self.pixelArt { 2.0 } else { 1.0 }
    }
}
