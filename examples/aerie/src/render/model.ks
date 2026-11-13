module Aerie

import wgpu.(Device, Buffer, BindGroup, RenderPipeline, RenderPass, ShaderModule, PipelineLayout, ColorTarget, VertexBufferLayout, DepthTest, BindingLayout)
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

    /// The cracks over block (x, y, z) as it's broken, `progress` 0 to 1.
    static func crack(x: Int64, y: Int64, z: Int64, progress: Float32) -> ModelBox {
        let stage = Int64(exactly: (progress * Float32(from: Layer.crackStages)).trunc()) ?? 0;
        let layer = Layer.Crack0.number + Float32(from: stage.clamp(0, Layer.crackStages - 1));
        // A hair bigger than the block, so the cracks sit on its faces.
        let half: Float32 = 0.503;
        ModelBox(
            center: Vec4(x: Float32(from: x) + 0.5, y: Float32(from: y) + 0.5, z: Float32(from: z) + 0.5, w: layer),
            axisX: Vec4(x: half, y: 0.0, z: 0.0, w: layer),
            axisY: Vec4(x: 0.0, y: half, z: 0.0, w: layer),
            axisZ: Vec4(x: 0.0, y: 0.0, z: half, w: 0.0)
        )
    }
}

let maxModelBoxes: Int64 = 1024;

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
            center: self.point(center).extended(w: side.number),
            axisX: (self.direction(x) * (half.x * s)).extended(w: top.number),
            axisY: (self.direction(y) * (half.y * s)).extended(w: bottom.number),
            axisZ: (self.direction(z) * (half.z * s)).extended(w: 0.0)
        )
    }

    /// A box lined up with the pose's own axes.
    func alignedBox(center: Vec3, half: Vec3, side side: Layer, top top: Layer, bottom bottom: Layer) -> ModelBox {
        self.box(center, half, x: Vec3.unitX, y: Vec3.unitY, z: Vec3.unitZ, side: side, top: top, bottom: bottom)
    }
}

/// Boxes at any angle (birds, and the cracks on a block being broken),
/// drawn from one instance buffer of up to `maxModelBoxes`.
struct ModelPass: not Copyable {
    var pipeline: RenderPipeline
    var shadowPipeline: RenderPipeline
    var buffer: Buffer

    /// `layout` is the world's group 0 alone, as the sky uses.
    static func create(device: Device, shader: ShaderModule, target target: DrawTarget, layout layout: PipelineLayout) -> ModelPass {
        let buffers = [VertexBufferLayout(packed: [.Float32x4, .Float32x4, .Float32x4, .Float32x4], stepMode: .Instance)];
        // Both faces are drawn: a mirrored wing turns its boxes inside out.
        let pipeline = device.createRenderPipeline(
            shader: shader,
            // Blended, for the cracks' soft lips; birds are opaque.
            targets: [ColorTarget(format: target.format, blend: .AlphaBlending)],
            buffers: buffers,
            vertexEntry: "vs_model",
            fragmentEntry: "fs_model",
            depth: .Some(DepthTest(format: depthFormat)),
            sampleCount: target.samples,
            layout: .Some(layout.share()),
            label: "models"
        );
        let shadowLayout = device.createPipelineLayout(groups: [[
            BindingLayout.uniformBuffer(0, visibility: [.Vertex])
        ]], label: "model shadow");
        let shadowPipeline = device.createRenderPipeline(
            shader: shader,
            targets: [],
            buffers: buffers,
            vertexEntry: "vs_model_shadow",
            depth: .Some(DepthTest(format: shadowFormat, depthBias: 2, depthBiasSlopeScale: 2.0)),
            layout: .Some(shadowLayout.share()),
            label: "model shadow"
        );
        let buffer = device.createBuffer(size: maxModelBoxes * Layout.of[ModelBox]().size, usage: [.Vertex, .CopyDst], label: "models");
        ModelPass(pipeline: pipeline, shadowPipeline: shadowPipeline, buffer: buffer)
    }

    /// Upload `boxes`, cut to what the buffer holds; returns how many.
    func write(device: Device, boxes boxes: [ModelBox]) -> Int64 {
        var kept = boxes;
        if kept.count > maxModelBoxes {
            kept.removeSubrange(maxModelBoxes..<kept.count);
        }
        if kept.count > 0 {
            device.write(self.buffer, data: kept);
        }
        kept.count
    }

    /// Draw the first `count` boxes written.
    func draw(pass: RenderPass, count count: Int64) {
        guard count > 0 else {
            return;
        }
        pass.setPipeline(self.pipeline);
        pass.setVertexBuffer(0, self.buffer);
        pass.draw(vertices: 36, instances: count);
    }

    /// Draw the first `count` boxes into the shadow map; `group` holds the
    /// uniforms.
    func drawShadows(pass: RenderPass, count count: Int64, group group: BindGroup) {
        guard count > 0 else {
            return;
        }
        pass.setPipeline(self.shadowPipeline);
        pass.setBindGroup(0, group);
        pass.setVertexBuffer(0, self.buffer);
        pass.draw(vertices: 36, instances: count);
    }
}
