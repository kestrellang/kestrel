// Mipmap generation: each level of a texture drawn from the one above it.

module wgpu

import wgpu.raw

/// A fullscreen triangle sampling the level above with a linear sampler,
/// which averages each 2x2 block of texels.
let mipmapShader: String = "
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var linearSampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    let uv = vec2f(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOut;
    out.position = vec4f(uv * vec2f(2.0, -2.0) + vec2f(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    return textureSample(source, linearSampler, in.uv);
}
";

extend Device {
    /// Fill mip levels 1 and on of every layer of `texture` from level 0,
    /// halving each time. The texture must be 2D, of a filterable color
    /// format, and made with `.TextureBinding` and `.RenderAttachment`
    /// (and `mipLevels` above 1, such as `Texture.fullMipChain`).
    public func generateMipmaps(texture: Texture) {
        let levels = texture.mipLevelCount;
        if levels < 2 {
            return;
        }
        let shader = self.createShaderModule(wgsl: mipmapShader, label: "mipmap");
        let pipeline = self.createRenderPipeline(shader: shader, targets: [ColorTarget(format: texture.format)], label: "mipmap");
        let sampler = self.createSampler(filter: .Linear, label: "mipmap");
        let layout = pipeline.bindGroupLayout(0);
        var layer: Int64 = 0;
        while layer < texture.layers {
            var level: Int64 = 1;
            while level < levels {
                let source = texture.createView(range: ViewRange.level(level - 1, layer: layer));
                let target = texture.createView(range: ViewRange.level(level, layer: layer));
                let group = self.createBindGroup(layout: layout, entries: [source.binding(0), sampler.binding(1)]);
                let encoder = self.createCommandEncoder();
                encoder.renderPass(color: [target.colorAttachment(clear: .None)]) { (pass) in
                    pass.setPipeline(pipeline);
                    pass.setBindGroup(0, group);
                    pass.draw(vertices: 3);
                };
                self.submit(encoder.finish());
                level = level + 1;
            }
            layer = layer + 1;
        }
    }
}
