module Aerie

/// The blocks you can place, keys 1 to 9, and which one is picked.
struct Hotbar {
    var blocks: [Block] = [.Driftstone, .Loam, .Mossloam, .Emberwood, .Planks, .Cloudglass, .Featherthatch, .Sailcloth, .Glowcap]
    var selected: Int64 = 0

    init() {}

    var block: Block { self.blocks(self.selected) }

    /// Pick from `controls`: a number key, then the wheel, which steps
    /// through the slots and wraps at the ends; away from you moves left.
    mutating func update(controls: Controls) {
        if controls.slotPressed >= 0 and controls.slotPressed < self.blocks.count {
            self.selected = controls.slotPressed;
        }
        let slots = self.blocks.count;
        self.selected = ((self.selected - controls.wheel) % slots + slots) % slots;
    }
}
