module Aerie

import wgpu.(Device, Buffer, BindGroup, RenderPipeline, RenderPass, ShaderModule, TextureFormat, ColorTarget, VertexBufferLayout, DepthTest, BindingLayout)
import linalg.(Vec4)

/// At most this many icon boxes at once: two per block on the hotbar is
/// plenty.
let maxIconBoxes: Int64 = 64;

/// Hotbar icons, rendered live: `ModelBox`es in screen pixels, textured
/// from the block atlas and drawn over the HUD.
struct IconPass: not Copyable {
    var pipeline: RenderPipeline
    /// Bind group 1: the window size.
    var group: BindGroup
    var screen: Buffer
    var buffer: Buffer

    /// `shader` is the block shader; group 0 is the world's bindings.
    static func create(device: Device, shader: ShaderModule, format format: TextureFormat) -> IconPass {
        let layout = device.createPipelineLayout(groups: [worldBindings(), [
            BindingLayout.uniformBuffer(0, visibility: [.Vertex])
        ]], label: "icons");
        let pipeline = device.createRenderPipeline(
            shader: shader,
            targets: [ColorTarget(format: format, blend: .AlphaBlending)],
            buffers: [VertexBufferLayout(packed: [.Float32x4, .Float32x4, .Float32x4, .Float32x4], stepMode: .Instance)],
            vertexEntry: "vs_icon",
            fragmentEntry: "fs_icon",
            depth: .Some(DepthTest(format: depthFormat)),
            sampleCount: samples,
            layout: .Some(layout.share()),
            label: "icons"
        );
        let screen = device.createBuffer(size: Layout.of[Vec4]().size, usage: [.Uniform, .CopyDst], label: "icon screen");
        let group = device.createBindGroup(layout: layout.bindGroupLayout(1), entries: [screen.binding(0)]);
        let buffer = device.createBuffer(size: maxIconBoxes * Layout.of[ModelBox]().size, usage: [.Vertex, .CopyDst], label: "icons");
        IconPass(pipeline: pipeline, group: group, screen: screen, buffer: buffer)
    }

    /// Upload `boxes` for a `width` by `height` window, cut to what the
    /// buffer holds; returns how many.
    func write(device: Device, boxes boxes: [ModelBox], width width: Int64, height height: Int64) -> Int64 {
        device.write(self.screen, value: Vec4(x: Float32(from: width), y: Float32(from: height), z: 0.0, w: 0.0));
        var kept = boxes;
        if kept.count > maxIconBoxes {
            kept.removeSubrange(maxIconBoxes..<kept.count);
        }
        if kept.count > 0 {
            device.write(self.buffer, data: kept);
        }
        kept.count
    }

    /// Draw the first `count` boxes written; `world` is bind group 0.
    func draw(pass: RenderPass, count count: Int64, world world: BindGroup) {
        guard count > 0 else {
            return;
        }
        pass.setPipeline(self.pipeline);
        pass.setBindGroup(0, world);
        pass.setBindGroup(1, self.group);
        pass.setVertexBuffer(0, self.buffer);
        pass.draw(vertices: 36, instances: count);
    }
}
