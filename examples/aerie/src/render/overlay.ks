module Aerie

import wgpu.(Device, Buffer, BindGroup, RenderPipeline, RenderPass, TextureView, Sampler, TextureFormat, ColorTarget, VertexBufferLayout, DepthTest, BindingLayout)
import linalg.(Vec4)

/// One overlay rectangle: `rect` is (x, y, width, height) in pixels from
/// the top left. `tile.x` is a layer to draw tinted by `color`, or negative
/// for a plain `color` fill; `tile.y` picks the block atlas (0), the
/// interface atlas smoothed (1), or the interface atlas crisp, for pixel
/// art (2); `tile.z` to `tile.w` is the span of the layer's
/// width to show, so a sprite can be drawn in stretched slices.
struct OverlayQuad: FFISafe {
    var rect: Vec4
    var color: Vec4
    var tile: Vec4
}

/// Text is one quad per run of lit dots, so a few lines take a thousand or more.
let maxOverlayQuads: Int64 = 4096;

/// The 2D interface, drawn last in the main pass over everything: it keeps
/// the pass's depth format but never tests or writes depth.
struct OverlayPass: not Copyable {
    var pipeline: RenderPipeline
    var group: BindGroup
    /// The window size in pixels, for the shader to place quads.
    var screen: Buffer
    var buffer: Buffer

    /// `blocks` and `ui` are the block and interface atlases.
    static func create(device: Device, format format: TextureFormat, encode encode: Bool, blocks blocks: TextureView, blockSampler blockSampler: Sampler, ui ui: TextureView, uiSampler uiSampler: Sampler) -> OverlayPass {
        let layout = device.createPipelineLayout(groups: [[
            BindingLayout.uniformBuffer(0, visibility: [.Vertex]),
            BindingLayout.texture(1, visibility: [.Fragment], dimension: .D2Array),
            BindingLayout.sampler(2, visibility: [.Fragment]),
            BindingLayout.texture(3, visibility: [.Fragment], dimension: .D2Array),
            BindingLayout.sampler(4, visibility: [.Fragment])
        ]], label: "overlay");
        let shader = device.createShaderModule(wgsl: loadShader("overlay", encode: encode), label: "overlay");
        let pipeline = device.createRenderPipeline(
            shader: shader,
            targets: [ColorTarget(format: format, blend: .AlphaBlending)],
            buffers: [VertexBufferLayout(packed: [.Float32x4, .Float32x4, .Float32x4], stepMode: .Instance)],
            depth: .Some(DepthTest(format: depthFormat, compare: .Always, write: false)),
            sampleCount: samples,
            layout: .Some(layout.share()),
            label: "overlay"
        );
        let screen = device.createBuffer(size: 16, usage: [.Uniform, .CopyDst], label: "screen");
        let group = device.createBindGroup(
            layout: layout.bindGroupLayout(0),
            entries: [screen.binding(0), blocks.binding(1), blockSampler.binding(2), ui.binding(3), uiSampler.binding(4)]
        );
        let buffer = device.createBuffer(size: maxOverlayQuads * Layout.of[OverlayQuad]().size, usage: [.Vertex, .CopyDst], label: "overlay");
        OverlayPass(pipeline: pipeline, group: group, screen: screen, buffer: buffer)
    }

    /// Upload `quads` for a `width` by `height` window, cut to what the
    /// buffer holds; returns how many.
    func write(device: Device, quads quads: [OverlayQuad], width width: Int64, height height: Int64) -> Int64 {
        device.write(self.screen, value: Vec4(x: Float32(from: width), y: Float32(from: height), z: 0.0, w: 0.0));
        var kept = quads;
        if kept.count > maxOverlayQuads {
            kept.removeSubrange(maxOverlayQuads..<kept.count);
        }
        if kept.count > 0 {
            device.write(self.buffer, data: kept);
        }
        kept.count
    }

    /// Draw the first `count` quads written, over everything.
    func draw(pass: RenderPass, count count: Int64) {
        guard count > 0 else {
            return;
        }
        pass.setPipeline(self.pipeline);
        pass.setBindGroup(0, self.group);
        pass.setVertexBuffer(0, self.buffer);
        pass.draw(vertices: 6, instances: count);
    }
}
