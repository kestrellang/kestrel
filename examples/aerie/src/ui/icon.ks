module Aerie

import linalg.(Vec3, Vec4)

/// The tilt of every hotbar icon, in radians: looking down on the block,
/// so its top shows.
let iconTilt: Float32 = 0.52;

/// How to draw a block as an icon: centered on (centerX, centerY) in
/// screen pixels, about `size` pixels across, turned `turn` radians about
/// its vertical axis.
struct IconPose {
    var centerX: Float32
    var centerY: Float32
    var size: Float32
    var turn: Float32

    /// Local direction `v` (x right, y up, z toward the viewer) turned and
    /// tilted, in screen pixels (y down).
    func direction(v: Vec3) -> Vec3 {
        let (c, s) = (self.turn.cos(), self.turn.sin());
        let turned = Vec3(x: v.x * c + v.z * s, y: v.y, z: -v.x * s + v.z * c);
        let (tc, ts) = (iconTilt.cos(), iconTilt.sin());
        let tilted = Vec3(x: turned.x, y: turned.y * tc - turned.z * ts, z: turned.y * ts + turned.z * tc);
        Vec3(x: tilted.x, y: -tilted.y, z: tilted.z)
    }

    /// A box with half-extents `half` along local axes `x`, `y` and `z`.
    func box(half: Vec3, x x: Vec3, y y: Vec3, z z: Vec3, side side: Layer, top top: Layer, bottom bottom: Layer) -> ModelBox {
        ModelBox(
            center: Vec4(x: self.centerX, y: self.centerY, z: 0.0, w: side.number),
            axisX: (self.direction(x) * half.x).extended(w: top.number),
            axisY: (self.direction(y) * half.y).extended(w: bottom.number),
            axisZ: (self.direction(z) * half.z).extended(w: 0.0)
        )
    }

    /// `block` as boxes: a cube, or a plant's two crossed cards.
    func boxes(block: Block) -> [ModelBox] {
        // A cube's corners reach about 1.4 half-sizes out once turned and
        // tilted, so this fills `size`.
        let half = self.size * 0.36;
        match block.shape {
            .Cross => {
                let layer = block.layer(.Cross0);
                let card = Vec3(x: half * 1.2, y: half * 1.2, z: 0.01);
                let diagonal = Vec3(x: 0.7071, y: 0.0, z: 0.7071);
                let across = Vec3(x: -0.7071, y: 0.0, z: 0.7071);
                [
                    self.box(card, x: diagonal, y: Vec3.unitY, z: across, side: layer, top: layer, bottom: layer),
                    self.box(card, x: across, y: Vec3.unitY, z: diagonal, side: layer, top: layer, bottom: layer)
                ]
            },
            _ => [self.box(Vec3(splat: half), x: Vec3.unitX, y: Vec3.unitY, z: Vec3.unitZ, side: block.layer(.South), top: block.layer(.Up), bottom: block.layer(.Down))]
        }
    }
}
