module Aerie

import linalg.(Vec4)

/// Hotbar slots are a whole multiple of the sprites' 24 pixels, so the
/// pixel art stays crisp.
let slotSize: Float32 = 72.0;
let slotGap: Float32 = 4.0;
/// From the hotbar's slots to the screen's bottom edge.
let hotbarBottom: Float32 = 22.0;
/// How far the wooden tray reaches past the slots, across and down.
let trayRimX: Float32 = 22.0;
let trayRimY: Float32 = 12.0;

/// The interface for a screen `width` by `height` pixels: the HUD during
/// play, and the title screen (see title.ks).
struct Hud {
    var width: Float32
    var height: Float32

    init(width: Int64, height: Int64) {
        self.width = Float32(from: width);
        self.height = Float32(from: height);
    }

    /// The crosshair, the hotbar and the selected block's name, `time`
    /// seconds in.
    func play(hotbar: Hotbar, gliding gliding: Bool, time time: Float32) -> [OverlayQuad] {
        var quads = self.crosshair(gliding);
        quads.append(contentsOf: self.hotbar(hotbar, time: time));
        quads
    }

    /// A white cross on a soft outline; it widens while gliding.
    func crosshair(gliding: Bool) -> [OverlayQuad] {
        let arm: Float32 = if gliding { 13.0 } else { 8.0 };
        let (cx, cy) = ((self.width * 0.5).floor(), (self.height * 0.5).floor());
        let outline = faded(ink, 0.45);
        let cross = faded(white, 0.9);
        [
            OverlayQuad.fill(cx - arm - 1.0, cy - 2.0, arm * 2.0 + 2.0, 4.0, outline),
            OverlayQuad.fill(cx - 2.0, cy - arm - 1.0, 4.0, arm * 2.0 + 2.0, outline),
            OverlayQuad.fill(cx - arm, cy - 1.0, arm * 2.0, 2.0, cross),
            OverlayQuad.fill(cx - 1.0, cy - arm, 2.0, arm * 2.0, cross)
        ]
    }

    /// Where the hotbar's slots sit.
    func hotbarLayout(hotbar: Hotbar) -> HotbarLayout {
        let count = Float32(from: hotbar.blocks.count);
        let total = count * slotSize + (count - 1.0) * slotGap;
        HotbarLayout(left: ((self.width - total) * 0.5).floor(), top: self.height - slotSize - hotbarBottom, width: total)
    }

    /// A wooden tray of stone slots and the selected block's name floating
    /// above. The blocks in the slots are 3D, drawn by `icons`.
    func hotbar(hotbar: Hotbar, time time: Float32) -> [OverlayQuad] {
        let layout = self.hotbarLayout(hotbar);
        var quads = OverlayQuad.sliced(.Panel, layout.left - trayRimX, layout.top - trayRimY, layout.width + trayRimX * 2.0, slotSize + trayRimY * 2.0, white);
        for i in 0..<hotbar.blocks.count {
            quads.append(contentsOf: layout.slotQuads(i, chosen: i == hotbar.selected));
        }
        let name = Label(text: hotbar.block.name, size: smallText, color: white);
        quads.append(contentsOf: name.onBoard(.Panel, width: self.width, y: layout.top - slotSize * 0.75, roomy: 1.0));
        quads
    }

    /// The hotbar's blocks as little 3D models, one in each slot, `time`
    /// seconds in: turned to show three faces, and the selected one bigger,
    /// bobbing and slowly spinning.
    func icons(hotbar: Hotbar, time time: Float32) -> [ModelBox] {
        let layout = self.hotbarLayout(hotbar);
        var boxes: [ModelBox] = [];
        for i in 0..<hotbar.blocks.count {
            let chosen = i == hotbar.selected;
            let (x, y) = layout.slotCorner(i, chosen: chosen);
            let size = slotSize - iconInset(chosen) * 2.0;
            let bob = if chosen { ((time * 3.0).sin() * 2.0).floor() - 1.0 } else { 0.0 };
            let turn = if chosen { Float32.pi * 0.25 + time * 1.2 } else { Float32.pi * 0.25 };
            let pose = IconPose(centerX: x + slotSize * 0.5, centerY: y + slotSize * 0.5 + bob, size: size, turn: turn);
            boxes.append(contentsOf: pose.boxes(hotbar.blocks(i)));
        }
        boxes
    }
}

/// How far a slot's icon sits in from its edges, in pixels: less for the
/// chosen one, so it's bigger.
func iconInset(chosen: Bool) -> Float32 {
    if chosen { 12.0 } else { 14.0 }
}

/// Where the hotbar's slots sit on screen: the first one's left edge, the
/// slots' top, and the width of them all.
struct HotbarLayout {
    var left: Float32
    var top: Float32
    var width: Float32

    /// The top left corner of slot `i`; the chosen slot rises a little.
    func slotCorner(i: Int64, chosen chosen: Bool) -> (Float32, Float32) {
        let lift: Float32 = if chosen { 6.0 } else { 0.0 };
        (self.left + Float32(from: i) * (slotSize + slotGap), self.top - lift)
    }

    /// Slot `i`'s frame; the chosen one turns gold, in a warm glow.
    func slotQuads(i: Int64, chosen chosen: Bool) -> [OverlayQuad] {
        let (x, y) = self.slotCorner(i, chosen: chosen);
        var quads: [OverlayQuad] = [];
        if chosen {
            let glow = slotSize * 0.4;
            quads.append(OverlayQuad.sprite(.Glow, x - glow, y - glow, slotSize + glow * 2.0, slotSize + glow * 2.0, Vec4(x: 1.0, y: 0.86, z: 0.55, w: 0.45)));
        }
        quads.append(OverlayQuad.sprite(if chosen { .SlotSelected } else { .Slot }, x, y, slotSize, slotSize, white));
        quads
    }
}
