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

    /// A wooden tray of stone slots, each with a little 3D block, and the
    /// selected block's name floating above.
    func hotbar(hotbar: Hotbar, time time: Float32) -> [OverlayQuad] {
        let count = Float32(from: hotbar.blocks.count);
        let total = count * slotSize + (count - 1.0) * slotGap;
        let left = ((self.width - total) * 0.5).floor();
        let top = self.height - slotSize - hotbarBottom;
        var quads = OverlayQuad.sliced(.Panel, left - trayRimX, top - trayRimY, total + trayRimX * 2.0, slotSize + trayRimY * 2.0, white);
        for i in 0..<hotbar.blocks.count {
            let x = left + Float32(from: i) * (slotSize + slotGap);
            quads.append(contentsOf: Hud.slot(hotbar.blocks(i), x: x, top: top, chosen: i == hotbar.selected, time: time));
        }
        let name = Label(text: hotbar.block.name, size: smallText, color: white);
        quads.append(contentsOf: name.onBoard(.Panel, width: self.width, y: top - slotSize * 0.75, roomy: 1.0));
        quads
    }

    /// One slot holding `block`, its left edge at x. The chosen slot turns
    /// gold and rises with a warm glow, its bigger icon bobbing.
    static func slot(block: Block, x x: Float32, top top: Float32, chosen chosen: Bool, time time: Float32) -> [OverlayQuad] {
        let lift: Float32 = if chosen { 6.0 } else { 0.0 };
        let inset: Float32 = if chosen { 10.0 } else { 14.0 };
        let bob = if chosen { ((time * 3.0).sin() * 2.0).floor() - 1.0 } else { 0.0 };
        let y = top - lift;
        var quads: [OverlayQuad] = [];
        if chosen {
            let glow = slotSize * 0.4;
            quads.append(OverlayQuad.sprite(.Glow, x - glow, y - glow, slotSize + glow * 2.0, slotSize + glow * 2.0, Vec4(x: 1.0, y: 0.86, z: 0.55, w: 0.45)));
        }
        quads.append(OverlayQuad.sprite(if chosen { .SlotSelected } else { .Slot }, x, y, slotSize, slotSize, white));
        let icon = slotSize - inset * 2.0;
        quads.append(OverlayQuad.ui(uiIconLayer(block.id), x + inset, y + inset + bob, icon, icon, white));
        quads
    }
}
