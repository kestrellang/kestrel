module Aerie

import linalg.(Vec3, Vec4)

/// One textured box anywhere in the world, at any angle: the model
/// pipeline draws it as 36 vertices from its center and three half-axes.
/// The layers ride in the w components: the sides in `center.w`, the top
/// (+y) face in `axisX.w` and the bottom in `axisY.w`.
struct ModelBox: FFISafe {
    var center: Vec4
    var axisX: Vec4
    var axisY: Vec4
    var axisZ: Vec4
}

let modelBoxSize: Int64 = 64;
let maxModelBoxes: Int64 = 1024;

func layerNumber(layer: Layer) -> Float32 {
    Float32(from: Int64(from: layer.index))
}

/// A place and an orientation to build a model in: local x is right, y is
/// up and z is forward, each `scale` blocks.
struct Pose {
    var origin: Vec3
    var right: Vec3
    var up: Vec3
    var forward: Vec3
    var scale: Float32

    /// Local direction `v` in the world, unscaled.
    func direction(v: Vec3) -> Vec3 {
        self.right * v.x + self.up * v.y + self.forward * v.z
    }

    /// Local point `v` in the world.
    func point(v: Vec3) -> Vec3 {
        self.origin + self.direction(v) * self.scale
    }

    /// A box centered on local `center`, `half` its size along local axes
    /// `x`, `y` and `z` (unit length, at right angles).
    func box(center: Vec3, half: Vec3, x x: Vec3, y y: Vec3, z z: Vec3, side side: Layer, top top: Layer, bottom bottom: Layer) -> ModelBox {
        let s = self.scale;
        ModelBox(
            center: self.point(center).extended(w: layerNumber(side)),
            axisX: (self.direction(x) * (half.x * s)).extended(w: layerNumber(top)),
            axisY: (self.direction(y) * (half.y * s)).extended(w: layerNumber(bottom)),
            axisZ: (self.direction(z) * (half.z * s)).extended(w: 0.0)
        )
    }

    /// A box lined up with the pose's own axes.
    func alignedBox(center: Vec3, half: Vec3, side side: Layer, top top: Layer, bottom bottom: Layer) -> ModelBox {
        self.box(center, half, x: Vec3.unitX, y: Vec3.unitY, z: Vec3.unitZ, side: side, top: top, bottom: bottom)
    }
}

/// The cracks over block (x, y, z) as it's broken, `progress` 0 to 1.
func crackBox(x: Int64, y: Int64, z: Int64, progress: Float32) -> ModelBox {
    let stage = (progress * Float32(from: Layer.crackStages)).toInt64() ?? 0;
    let layer = layerNumber(.Crack0) + Float32(from: stage.clamp(0, Layer.crackStages - 1));
    // A hair bigger than the block, so the cracks sit on its faces.
    let half: Float32 = 0.503;
    ModelBox(
        center: Vec4(x: Float32(from: x) + 0.5, y: Float32(from: y) + 0.5, z: Float32(from: z) + 0.5, w: layer),
        axisX: Vec4(x: half, y: 0.0, z: 0.0, w: layer),
        axisY: Vec4(x: 0.0, y: half, z: 0.0, w: layer),
        axisZ: Vec4(x: 0.0, y: 0.0, z: half, w: 0.0)
    )
}
