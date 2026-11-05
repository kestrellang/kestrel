module Aerie

import linalg.(Vec4)

/// Where twinkles sit around the logo, as fractions of its width and of
/// the text's height, and each one's phase in its beat.
let twinkleSpots: [(Float32, Float32, Float32)] = [(0.08, 0.15, 0.0), (0.95, 0.2, 1.7), (0.25, -0.12, 3.1), (0.78, -0.1, 4.4), (0.5, 1.05, 2.3), (0.1, 0.9, 5.2), (0.92, 0.88, 0.9)];

/// Where the title's logo sits: a grid of `uiLogoColumns` by `uiLogoRows`
/// square cells, each `cell` pixels.
struct LogoLayout {
    var left: Float32
    var top: Float32
    var cell: Float32

    var width: Float32 { self.cell * Float32(from: uiLogoColumns) }
    var height: Float32 { self.cell * Float32(from: uiLogoRows) }
    /// The height of the lettering inside the logo, and where it starts.
    var textSize: Float32 { self.height * 0.62 }
    var textTop: Float32 { self.top + self.height * 0.14 }
}

extend Hud {
    /// The title screen, `time` seconds in: the name, bobbing in a soft
    /// glow with twinkles around it, and a Play Now button.
    func title(time: Float32) -> [OverlayQuad] {
        let logo = self.logoLayout(time);
        var quads = self.logo(logo);
        quads.append(contentsOf: Hud.twinkles(logo, time: time));
        quads.append(contentsOf: self.playButton(logo, time: time));
        quads
    }

    /// The logo about 80% of the screen across, bobbing gently.
    func logoLayout(time: Float32) -> LogoLayout {
        let across = (self.width * 0.8).clamp(360.0, 1500.0).floor();
        let cell = (across / Float32(from: uiLogoColumns)).floor();
        let bob = (time * 1.3).sin() * cell * 0.05;
        let width = cell * Float32(from: uiLogoColumns);
        LogoLayout(left: ((self.width - width) * 0.5).floor(), top: (self.height * 0.12 + bob).floor(), cell: cell)
    }

    /// The pixel font's letters, each pixel a driftstone block capped with
    /// moss where open to the sky (see tools/make_ui.py), over a wide soft
    /// glow so they read over any sky.
    func logo(logo: LogoLayout) -> [OverlayQuad] {
        let cell = logo.cell;
        var quads = [OverlayQuad.sprite(.Glow, self.width * 0.5 - logo.width * 0.75, logo.top - cell * 0.6, logo.width * 1.5, logo.height + cell * 1.2, Vec4(x: 0.16, y: 0.18, z: 0.36, w: 0.4))];
        for i in 0..<(uiLogoRows * uiLogoColumns) {
            let (c, r) = (Float32(from: i % uiLogoColumns), Float32(from: i / uiLogoColumns));
            quads.append(OverlayQuad.ui(uiLogoLayer + Float32(from: i), logo.left + c * cell, logo.top + r * cell, cell, cell, white));
        }
        quads
    }

    /// Twinkles around the name, each on its own beat.
    static func twinkles(logo: LogoLayout, time time: Float32) -> [OverlayQuad] {
        let size = logo.textSize;
        var quads: [OverlayQuad] = [];
        for (fx, fy, phase) in twinkleSpots {
            let beat = (time * 2.2 + phase).sin() * 0.5 + 0.5;
            let s = size * (0.18 + beat * 0.22);
            let x = logo.left + logo.width * fx;
            let y = logo.textTop + size * 0.8 * fy;
            quads.append(OverlayQuad.sprite(.Twinkle, x - s * 0.5, y - s * 0.5, s, s, Vec4(x: 1.0, y: 0.97, z: 0.88, w: 0.4 + beat * 0.6)));
        }
        quads
    }

    /// "Play Now" on a big warm wooden button, the one thing to press,
    /// with a glow that breathes.
    func playButton(logo: LogoLayout, time time: Float32) -> [OverlayQuad] {
        let size = Label.crisp(Label.crisp((logo.textSize * 0.2).clamp(18.0, 30.0)) * 2.7);
        let label = Label(text: "Play Now", size: size, color: white);
        let y = (self.height * 0.68).floor();
        let glowWidth = (label.width + size * 3.4) * 1.4;
        let pulse = 0.5 + 0.5 * (time * 3.0).sin();
        var quads = [OverlayQuad.sprite(.Glow, (self.width - glowWidth) * 0.5, y - size * 2.2, glowWidth, size * 5.0, Vec4(x: 1.0, y: 0.82, z: 0.5, w: 0.15 + pulse * 0.2))];
        quads.append(contentsOf: label.onBoard(.Button, width: self.width, y: y, roomy: 1.1));
        quads
    }
}
