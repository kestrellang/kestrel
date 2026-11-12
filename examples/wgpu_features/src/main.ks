// Checks each wgpu feature beyond the basics on the GPU, offscreen, and
// prints one line per check. Exits with the number that failed.
//
// Most checks draw quads into a 16x16 RGBA8 target and read pixels back.
// A quad is a rectangle in pixels with a color and a depth.

module Main

import wgpu.(Instance, Adapter, Device, Buffer, Texture, TextureFormat, TextureUsage, ShaderModule, WgpuError, ColorTarget, VertexBufferLayout, VertexAttribute, VertexFormat, Color, DepthTest, StencilTest, StencilFace, BlendMode, BlendComponent, ColorChannel, CompareFunction, Feature, BindingLayout, PushConstantRange, ShaderStage, RenderPipeline, RenderPassDescriptor, ViewRange, DrawIndirectArgs, BundleTargets, QueryType, RenderPass)
import linalg.(Vec4)

let size: Int64 = 16;
let colorFormat: TextureFormat = .RGBA8Unorm;

/// A rectangle (`rect` is left, top, right, bottom in clip space), its
/// color and depth, laid out as the WGSL `Quad` (48 bytes).
struct Quad: FFISafe {
    var rect: Vec4
    var color: Vec4
    var depth: Float32
    var pad0: Float32
    var pad1: Float32
    var pad2: Float32
}

/// Pixel column `px` in clip space.
func clipX(px: Int64) -> Float32 { Float32(rounding: px) / (Float32(rounding: size) / 2.0) - 1.0 }

/// Pixel row `py` in clip space, where y points up.
func clipY(py: Int64) -> Float32 { 1.0 - Float32(rounding: py) / (Float32(rounding: size) / 2.0) }

/// A quad over pixels `left` to `right` and `top` to `bottom` of the target.
func quad(left: Int64, top: Int64, right: Int64, bottom: Int64, color color: Vec4, depth depth: Float32 = 0.5) -> Quad {
    Quad(rect: Vec4(x: clipX(left), y: clipY(top), z: clipX(right), w: clipY(bottom)), color: color, depth: depth, pad0: 0.0, pad1: 0.0, pad2: 0.0)
}

let red: Vec4 = Vec4(x: 1.0, y: 0.0, z: 0.0, w: 1.0);
let green: Vec4 = Vec4(x: 0.0, y: 1.0, z: 0.0, w: 1.0);
let blue: Vec4 = Vec4(x: 0.0, y: 0.0, z: 1.0, w: 1.0);
let white: Vec4 = Vec4(splat: 1.0);
let halfRed: Vec4 = Vec4(x: 0.5, y: 0.0, z: 0.0, w: 1.0);

/// WGSL shared by the quad shaders: the corner `i` of a quad's two
/// triangles.
let corner: String = "
struct Quad { rect: vec4f, color: vec4f, depth: f32 };

fn corner(i: u32, rect: vec4f) -> vec2f {
    var c = array<vec2f, 6>(vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0), vec2f(0.0, 1.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0));
    let t = c[i % 6u];
    return vec2f(mix(rect.x, rect.z, t.x), mix(rect.y, rect.w, t.y));
}
";

/// Quads from a per-instance vertex buffer of `Quad`s.
let instanceShader: String = corner + "
struct Out { @builtin(position) position: vec4f, @location(0) color: vec4f };

@vertex
fn vs_main(@builtin(vertex_index) i: u32, @location(0) rect: vec4f, @location(1) color: vec4f, @location(2) depth: f32) -> Out {
    var out: Out;
    out.position = vec4f(corner(i, rect), depth, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4f { return in.color; }
";

/// One quad from a uniform buffer.
let uniformShader: String = corner + "
@group(0) @binding(0) var<uniform> quad: Quad;

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    return vec4f(corner(i, quad.rect), quad.depth, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f { return quad.color; }
";

/// One quad from push constants.
let pushShader: String = corner + "
var<push_constant> quad: Quad;

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    return vec4f(corner(i, quad.rect), quad.depth, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f { return quad.color; }
";

/// A fullscreen triangle whose fragments read a depth texture through a
/// comparison sampler: 1 where `0.75 < depth`, else 0.
let compareShader: String = "
@group(0) @binding(0) var depthTexture: texture_depth_2d;
@group(0) @binding(1) var shadowSampler: sampler_comparison;

struct Out { @builtin(position) position: vec4f, @location(0) uv: vec2f };

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Out {
    let uv = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
    var out: Out;
    out.position = vec4f(uv * vec2f(2.0, -2.0) + vec2f(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4f {
    let lit = textureSampleCompare(depthTexture, shadowSampler, in.uv, 0.75);
    return vec4f(lit, 0.0, 0.0, 1.0);
}
";

/// A fullscreen triangle sampling a cube map along +X, +Y and -Z, one per
/// third of the target's width.
let cubeShader: String = "
@group(0) @binding(0) var cube: texture_cube<f32>;
@group(0) @binding(1) var cubeSampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let uv = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4f(uv * vec2f(2.0, -2.0) + vec2f(-1.0, 1.0), 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) p: vec4f) -> @location(0) vec4f {
    var dir = vec3f(1.0, 0.0, 0.0);
    if p.x > 6.0 { dir = vec3f(0.0, 1.0, 0.0); }
    if p.x > 11.0 { dir = vec3f(0.0, 0.0, -1.0); }
    return textureSampleLevel(cube, cubeSampler, dir, 0.0);
}
";

/// Counts its workgroups into `count`.
let countShader: String = "
@group(0) @binding(0) var<storage, read_write> count: atomic<u32>;

@compute @workgroup_size(1)
fn main() {
    atomicAdd(&count, 1u);
}
";

// ===== Helpers =====

/// How the instance shader reads a buffer of `Quad`s.
func quadLayout() -> VertexBufferLayout {
    VertexBufferLayout(stride: 48, attributes: [
        VertexAttribute(format: .Float32x4, offset: 0, location: 0),
        VertexAttribute(format: .Float32x4, offset: 16, location: 1),
        VertexAttribute(format: .Float32, offset: 32, location: 2)
    ], stepMode: .Instance)
}

func target(device: Device) -> Texture {
    device.createTexture(width: size, height: size, format: colorFormat, usage: [.RenderAttachment, .CopySrc], label: "target")
}

/// The RGBA bytes of pixel (`x`, `y`) of a 16-wide readback.
func pixel(bytes: [UInt8], x: Int64, y: Int64) -> (Int64, Int64, Int64, Int64) {
    let i = (y * size + x) * 4;
    (Int64(from: bytes(i)), Int64(from: bytes(i + 1)), Int64(from: bytes(i + 2)), Int64(from: bytes(i + 3)))
}

func near(a: Int64, b: Int64) -> Bool {
    let d = a - b;
    d >= -3 and d <= 3
}

func isColor(p: (Int64, Int64, Int64, Int64), r: Int64, g: Int64, b: Int64) -> Bool {
    let (pr, pg, pb, _) = p;
    near(pr, r) and near(pg, g) and near(pb, b)
}

func show(p: (Int64, Int64, Int64, Int64)) -> String {
    let (r, g, b, a) = p;
    "(\(r), \(g), \(b), \(a))"
}

/// Print the outcome of check `name` and count it if it failed.
func report(name: String, ok: Bool, detail: String) -> Int64 {
    if ok {
        println("ok    \(name)");
        return 0;
    }
    println("FAIL  \(name): \(detail)");
    1
}

// ===== Checks =====

/// A 4x multisampled triangle resolved into a plain texture: pixels on its
/// edge blend with the background.
func checkMultisampling(device: Device, shader: ShaderModule) -> Int64 throws WgpuError {
    let samples = device.createTexture(width: size, height: size, format: colorFormat, usage: [.RenderAttachment], sampleCount: 4, label: "msaa");
    let resolved = target(device);
    let pipeline = device.createRenderPipeline(shader: shader, targets: [ColorTarget(format: colorFormat)], buffers: [quadLayout()], sampleCount: 4);
    // A quad slanted by its corners isn't possible with `Quad`, so cover a
    // rectangle with fractional edges: x from 2.5 to 13.5 pixels.
    var q = quad(0, 0, 16, 16, color: white);
    q.rect = Vec4(x: -0.6875, y: 1.0, z: 0.6875, w: -1.0);
    let quads = device.createBuffer(data: [q], usage: [.Vertex]);
    let samplesView = samples.createView();
    let resolvedView = resolved.createView();
    let encoder = device.createCommandEncoder();
    encoder.renderPass(color: [samplesView.colorAttachment(resolvingInto: resolvedView)]) { (pass) in
        pass.setPipeline(pipeline);
        pass.setVertexBuffer(0, quads);
        pass.draw(vertices: 6);
    };
    device.submit(encoder.finish());
    let bytes = try device.read(texture: resolved);
    let (edge, _, _, _) = pixel(bytes, 2, 8);
    let (inside, _, _, _) = pixel(bytes, 8, 8);
    .Ok(report("multisampling", edge > 20 and edge < 235 and inside == 255, "edge red \(edge), inside red \(inside)"))
}

/// A one-texel checkerboard's mip chain ends in a single gray texel.
func checkMipmaps(device: Device) -> Int64 throws WgpuError {
    let levels = Texture.fullMipChain(width: size, height: size);
    let texture = device.createTexture(width: size, height: size, format: colorFormat, usage: [.TextureBinding, .RenderAttachment, .CopyDst, .CopySrc], mipLevels: levels, label: "mips");
    var texels: [UInt8] = [];
    var i = 0;
    while i < size * size {
        let v: UInt8 = if (i % size + i / size) % 2 == 0 { 255 } else { 0 };
        texels.append(v);
        texels.append(v);
        texels.append(v);
        texels.append(255);
        i = i + 1;
    }
    device.write(texture: texture, data: texels);
    device.generateMipmaps(texture);
    let last = try device.read(texture: texture, mipLevel: levels - 1);
    let gray = Int64(from: last(0));
    .Ok(report("mipmaps", levels == 5 and texture.mipLevelCount == 5 and gray > 110 and gray < 145, "\(levels) levels, last texel \(gray)"))
}

/// Writing one layer of an array texture and copying it to another texture.
func checkLayersAndCopies(device: Device) -> Int64 throws WgpuError {
    let array = device.createTexture(width: size, height: size, format: colorFormat, usage: [.CopyDst, .CopySrc], layers: 3, label: "array");
    var texels: [UInt8] = [];
    var i = 0;
    while i < size * size {
        texels.append(10);
        texels.append(20);
        texels.append(30);
        texels.append(255);
        i = i + 1;
    }
    device.write(texture: array, data: texels, layer: 2);
    let copy = device.createTexture(width: size, height: size, format: colorFormat, usage: [.CopyDst, .CopySrc], label: "copy");
    let encoder = device.createCommandEncoder();
    encoder.copy(texture: array, toTexture: copy, fromLayer: 2);
    device.submit(encoder.finish());
    let bytes = try device.read(texture: copy);
    let p = pixel(bytes, 5, 5);
    .Ok(report("array layers and copies", array.layers == 3 and isColor(p, 10, 20, 30), "copied \(show(p))"))
}

/// A depth-only pass, then a comparison sampler reading its depth.
func checkComparisonSampler(device: Device, shader: ShaderModule) -> Int64 throws WgpuError {
    let depth = device.createTexture(width: size, height: size, format: .Depth32Float, usage: [.RenderAttachment, .TextureBinding], label: "depth");
    let depthPipeline = device.createRenderPipeline(shader: shader, targets: [], buffers: [quadLayout()], depth: .Some(DepthTest(format: .Depth32Float)));
    let quads = device.createBuffer(data: [quad(4, 4, 12, 12, color: white, depth: 0.5)], usage: [.Vertex]);
    let compareModule = device.createShaderModule(wgsl: compareShader);
    let comparePipeline = device.createRenderPipeline(shader: compareModule, targets: [ColorTarget(format: colorFormat)]);
    let sampler = device.createSampler(filter: .Nearest, compare: .Some(.Less));
    let depthView = depth.createView();
    let group = device.createBindGroup(layout: comparePipeline.bindGroupLayout(0), entries: [depthView.binding(0), sampler.binding(1)]);
    let color = target(device);
    let colorView = color.createView();
    let encoder = device.createCommandEncoder();
    encoder.renderPass(color: [], depth: depthView.depthAttachment()) { (pass) in
        pass.setPipeline(depthPipeline);
        pass.setVertexBuffer(0, quads);
        pass.draw(vertices: 6);
    };
    encoder.renderPass(color: [colorView.colorAttachment()]) { (pass) in
        pass.setPipeline(comparePipeline);
        pass.setBindGroup(0, group);
        pass.draw(vertices: 3);
    };
    device.submit(encoder.finish());
    let bytes = try device.read(texture: color);
    let covered = pixel(bytes, 8, 8);
    let open = pixel(bytes, 1, 1);
    .Ok(report("depth-only pass and comparison sampler", isColor(covered, 0, 0, 0) and isColor(open, 255, 0, 0), "covered \(show(covered)), open \(show(open))"))
}

/// One pass writes stencil (and no color) in the middle; a second draw
/// over everything passes only there.
func checkStencilAndWriteMask(device: Device, shader: ShaderModule) -> Int64 throws WgpuError {
    let format: TextureFormat = .Depth24PlusStencil8;
    let stencil = device.createTexture(width: size, height: size, format: format, usage: [.RenderAttachment], label: "stencil");
    let mark = device.createRenderPipeline(
        shader: shader,
        targets: [ColorTarget(format: colorFormat, writeMask: [])],
        buffers: [quadLayout()],
        depth: .Some(DepthTest(format: format, compare: .Always, write: false, stencil: .Some(StencilTest(both: StencilFace(pass: .Replace)))))
    );
    let fill = device.createRenderPipeline(
        shader: shader,
        targets: [ColorTarget(format: colorFormat, writeMask: [.Green, .Alpha])],
        buffers: [quadLayout()],
        depth: .Some(DepthTest(format: format, compare: .Always, write: false, stencil: .Some(StencilTest(both: StencilFace(compare: .Equal)))))
    );
    let quads = device.createBuffer(data: [quad(4, 4, 12, 12, color: white), quad(0, 0, 16, 16, color: white)], usage: [.Vertex]);
    let color = target(device);
    let colorView = color.createView();
    let stencilView = stencil.createView();
    let encoder = device.createCommandEncoder();
    encoder.renderPass(color: [colorView.colorAttachment(clear: .Some(Color(r: 0.0, g: 0.0, b: 1.0)))], depth: stencilView.depthAttachment()) { (pass) in
        pass.setStencilReference(1);
        pass.setVertexBuffer(0, quads);
        pass.setPipeline(mark);
        pass.draw(vertices: 6, firstInstance: 0);
        pass.setPipeline(fill);
        pass.draw(vertices: 6, firstInstance: 1);
    };
    device.submit(encoder.finish());
    let bytes = try device.read(texture: color);
    // Inside: blue kept, green written (red masked off). Outside: blue only.
    let inside = pixel(bytes, 8, 8);
    let outside = pixel(bytes, 1, 1);
    .Ok(report("stencil test and write masks", isColor(inside, 0, 255, 255) and isColor(outside, 0, 0, 255), "inside \(show(inside)), outside \(show(outside))"))
}

/// Additive blending adds two half-red quads; a constant blend factor
/// takes its color from the pass.
func checkBlending(device: Device, shader: ShaderModule) -> Int64 throws WgpuError {
    let additive = device.createRenderPipeline(shader: shader, targets: [ColorTarget(format: colorFormat, blend: .Additive)], buffers: [quadLayout()]);
    let constant = device.createRenderPipeline(
        shader: shader,
        targets: [ColorTarget(format: colorFormat, blend: .Custom(BlendComponent(source: .Constant, destination: .Zero), BlendComponent(source: .One, destination: .Zero)))],
        buffers: [quadLayout()]
    );
    let quads = device.createBuffer(data: [quad(0, 0, 8, 16, color: halfRed), quad(8, 0, 16, 16, color: white)], usage: [.Vertex]);
    let color = target(device);
    let view = color.createView();
    let encoder = device.createCommandEncoder();
    encoder.renderPass(color: [view.colorAttachment()]) { (pass) in
        pass.setVertexBuffer(0, quads);
        pass.setPipeline(additive);
        pass.draw(vertices: 6, instances: 1);
        pass.draw(vertices: 6, instances: 1);
        pass.setPipeline(constant);
        pass.setBlendConstant(Color(r: 0.0, g: 0.5, b: 0.0));
        pass.draw(vertices: 6, firstInstance: 1);
    };
    device.submit(encoder.finish());
    let bytes = try device.read(texture: color);
    let added = pixel(bytes, 3, 8);
    let constantColor = pixel(bytes, 12, 8);
    .Ok(report("blend modes and blend constant", isColor(added, 255, 0, 0) and isColor(constantColor, 0, 128, 0), "added \(show(added)), constant \(show(constantColor))"))
}

/// An explicit pipeline layout with a dynamic uniform buffer: two quads
/// from one buffer at different offsets.
func checkLayoutsAndDynamicOffsets(device: Device) -> Int64 throws WgpuError {
    let stride = device.limits.minUniformBufferOffsetAlignment;
    let layout = device.createPipelineLayout(groups: [[BindingLayout.uniformBuffer(0, visibility: [.Vertex, .Fragment], dynamic: true)]]);
    let shaderModule = device.createShaderModule(wgsl: uniformShader);
    let pipeline = device.createRenderPipeline(shader: shaderModule, targets: [ColorTarget(format: colorFormat)], layout: .Some(layout.share()));
    let uniforms = device.createBuffer(size: stride * 2, usage: [.Uniform, .CopyDst]);
    device.write(uniforms, value: quad(0, 0, 8, 16, color: red));
    device.write(uniforms, value: quad(8, 0, 16, 16, color: green), offset: stride);
    let group = device.createBindGroup(layout: layout.bindGroupLayout(0), entries: [uniforms.binding(0, size: .Some(48))]);
    let color = target(device);
    let view = color.createView();
    let encoder = device.createCommandEncoder();
    encoder.renderPass(color: [view.colorAttachment()]) { (pass) in
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, group, dynamicOffsets: [0]);
        pass.draw(vertices: 6);
        pass.setBindGroup(0, group, dynamicOffsets: [stride]);
        pass.draw(vertices: 6);
    };
    device.submit(encoder.finish());
    let bytes = try device.read(texture: color);
    let left = pixel(bytes, 3, 8);
    let right = pixel(bytes, 12, 8);
    .Ok(report("pipeline layouts and dynamic offsets", isColor(left, 255, 0, 0) and isColor(right, 0, 255, 0), "left \(show(left)), right \(show(right))"))
}

/// Quad 0 drawn indirectly, quad 1 indexed from instance 1, quad 2 from
/// instance 2; quad 3 never.
func checkDrawCalls(device: Device, shader: ShaderModule) -> Int64 throws WgpuError {
    let pipeline = device.createRenderPipeline(shader: shader, targets: [ColorTarget(format: colorFormat)], buffers: [quadLayout()]);
    let quads = device.createBuffer(data: [
        quad(0, 0, 4, 16, color: red),
        quad(4, 0, 8, 16, color: green),
        quad(8, 0, 12, 16, color: blue),
        quad(12, 0, 16, 16, color: white)
    ], usage: [.Vertex]);
    let indices: [UInt16] = [0, 1, 2, 3, 4, 5];
    let indexBuffer = device.createBuffer(data: indices, usage: [.Index]);
    let args = device.createBuffer(data: [DrawIndirectArgs(vertices: 6)], usage: [.Indirect]);
    let color = target(device);
    let view = color.createView();
    let encoder = device.createCommandEncoder();
    encoder.renderPass(color: [view.colorAttachment()]) { (pass) in
        pass.setPipeline(pipeline);
        pass.setVertexBuffer(0, quads);
        pass.setIndexBuffer(indexBuffer, format: .Uint16);
        pass.drawIndirect(args);
        pass.drawIndexed(indices: 6, firstInstance: 1);
        pass.draw(vertices: 6, firstInstance: 2);
    };
    device.submit(encoder.finish());
    let bytes = try device.read(texture: color);
    let ok = isColor(pixel(bytes, 1, 8), 255, 0, 0)
        and isColor(pixel(bytes, 5, 8), 0, 255, 0)
        and isColor(pixel(bytes, 9, 8), 0, 0, 255)
        and isColor(pixel(bytes, 13, 8), 0, 0, 0);
    .Ok(report("indirect, indexed and instanced draws", ok, "row \(show(pixel(bytes, 1, 8))) \(show(pixel(bytes, 5, 8))) \(show(pixel(bytes, 9, 8))) \(show(pixel(bytes, 13, 8)))"))
}

/// A render bundle recorded once and replayed in a pass.
func checkRenderBundle(device: Device, shader: ShaderModule) -> Int64 throws WgpuError {
    let pipeline = device.createRenderPipeline(shader: shader, targets: [ColorTarget(format: colorFormat)], buffers: [quadLayout()]);
    let quads = device.createBuffer(data: [quad(4, 4, 12, 12, color: green)], usage: [.Vertex]);
    let bundle = device.createRenderBundle(BundleTargets(colorFormats: [colorFormat])) { (b) in
        b.setPipeline(pipeline);
        b.setVertexBuffer(0, quads);
        b.draw(vertices: 6);
    };
    let color = target(device);
    let view = color.createView();
    let encoder = device.createCommandEncoder();
    encoder.renderPass(color: [view.colorAttachment()]) { (pass) in
        pass.execute(bundle);
    };
    device.submit(encoder.finish());
    let bytes = try device.read(texture: color);
    let p = pixel(bytes, 8, 8);
    .Ok(report("render bundles", isColor(p, 0, 255, 0), "center \(show(p))"))
}

/// Occlusion queries count the samples an 8x8 quad covers, and none for a
/// quad hidden behind it.
func checkOcclusionQueries(device: Device, shader: ShaderModule) -> Int64 throws WgpuError {
    let pipeline = device.createRenderPipeline(shader: shader, targets: [ColorTarget(format: colorFormat)], buffers: [quadLayout()], depth: .Some(DepthTest()));
    let quads = device.createBuffer(data: [quad(4, 4, 12, 12, color: red, depth: 0.2), quad(4, 4, 12, 12, color: green, depth: 0.6)], usage: [.Vertex]);
    let queries = device.createQuerySet(kind: .Occlusion, count: 2);
    let depth = device.createTexture(width: size, height: size, format: .Depth24Plus, usage: [.RenderAttachment]);
    let color = target(device);
    let view = color.createView();
    let depthView = depth.createView();
    let encoder = device.createCommandEncoder();
    let descriptor = RenderPassDescriptor(color: [view.colorAttachment()], depth: .Some(depthView.depthAttachment()), occlusionQueries: .Some(queries.occlusionQueries));
    encoder.renderPass(descriptor) { (pass) in
        pass.setPipeline(pipeline);
        pass.setVertexBuffer(0, quads);
        pass.beginOcclusionQuery(0);
        pass.draw(vertices: 6, firstInstance: 0);
        pass.endOcclusionQuery();
        pass.beginOcclusionQuery(1);
        pass.draw(vertices: 6, firstInstance: 1);
        pass.endOcclusionQuery();
    };
    device.submit(encoder.finish());
    let counts = try device.readQueries(queries);
    let visible = Int64(exactly: counts(0))!;
    let hidden = Int64(exactly: counts(1))!;
    .Ok(report("occlusion queries", visible == 64 and hidden == 0, "visible \(visible), hidden \(hidden)"))
}

/// Timestamps around a compute pass and a render pass run forward.
func checkTimestamps(device: Device) -> Int64 throws WgpuError {
    if not device.supports(.TimestampQuery) {
        println("skip  timestamps: the adapter has no TimestampQuery");
        return .Ok(0);
    }
    let queries = device.createQuerySet(kind: .Timestamp, count: 4);
    let color = target(device);
    let view = color.createView();
    let encoder = device.createCommandEncoder();
    encoder.computePass(timestamps: queries.timestamps(beginning: .Some(0), end: .Some(1))) { (pass) in };
    encoder.renderPass(RenderPassDescriptor(color: [view.colorAttachment()], timestamps: .Some(queries.timestamps(beginning: .Some(2), end: .Some(3))))) { (pass) in };
    device.submit(encoder.finish());
    let times = try device.readQueries(queries);
    let ordered = times(0) <= times(1) and times(1) <= times(2) and times(2) <= times(3) and times(3) > 0;
    .Ok(report("timestamp queries", ordered, "times \(times(0)) \(times(1)) \(times(2)) \(times(3))"))
}

/// A quad whose rectangle and color come from push constants.
func checkPushConstants(device: Device) -> Int64 throws WgpuError {
    if not device.supports(.PushConstants) {
        println("skip  push constants: the adapter has no PushConstants");
        return .Ok(0);
    }
    let both: [ShaderStage] = [.Vertex, .Fragment];
    let layout = device.createPipelineLayout(groups: [], pushConstants: [PushConstantRange.first(48, stages: both)]);
    let shaderModule = device.createShaderModule(wgsl: pushShader);
    let pipeline = device.createRenderPipeline(shader: shaderModule, targets: [ColorTarget(format: colorFormat)], layout: .Some(layout));
    let color = target(device);
    let view = color.createView();
    let encoder = device.createCommandEncoder();
    encoder.renderPass(color: [view.colorAttachment()]) { (pass) in
        pass.setPipeline(pipeline);
        pass.setPushConstants(stages: both, value: quad(0, 0, 8, 8, color: blue));
        pass.draw(vertices: 6);
    };
    device.submit(encoder.finish());
    let bytes = try device.read(texture: color);
    let drawn = pixel(bytes, 2, 2);
    let blank = pixel(bytes, 12, 12);
    .Ok(report("push constants", isColor(drawn, 0, 0, 255) and isColor(blank, 0, 0, 0), "drawn \(show(drawn)), blank \(show(blank))"))
}

/// An indirect dispatch of 5 workgroups, each counting itself.
func checkIndirectDispatch(device: Device) -> Int64 throws WgpuError {
    let shaderModule = device.createShaderModule(wgsl: countShader);
    let pipeline = device.createComputePipeline(shader: shaderModule, entryPoint: "main");
    let zero: [UInt32] = [0];
    let count = device.createBuffer(data: zero, usage: [.Storage, .CopySrc]);
    let workgroups: [UInt32] = [5, 1, 1];
    let args = device.createBuffer(data: workgroups, usage: [.Indirect]);
    let readback = device.createBuffer(size: 4, usage: [.MapRead, .CopyDst]);
    let group = device.createBindGroup(layout: pipeline.bindGroupLayout(0), entries: [count.binding(0)]);
    let encoder = device.createCommandEncoder();
    encoder.computePass { (pass) in
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, group);
        pass.dispatchIndirect(args);
    };
    encoder.copy(from: count, to: readback, size: 4);
    device.submit(encoder.finish());
    let result: [UInt32] = try device.read(readback);
    .Ok(report("indirect dispatch", result(0) == 5, "counted \(result(0))"))
}

/// Six layers seen as a cube map, sampled along three axes.
func checkCubeMap(device: Device) -> Int64 throws WgpuError {
    let texture = device.createTexture(width: 4, height: 4, format: colorFormat, usage: [.TextureBinding, .CopyDst], layers: 6, label: "cube");
    // Face order: +X, -X, +Y, -Y, +Z, -Z.
    let faces: [(UInt8, UInt8, UInt8)] = [(255, 0, 0), (0, 0, 0), (0, 255, 0), (0, 0, 0), (0, 0, 0), (0, 0, 255)];
    var layer = 0;
    for (r, g, b) in faces {
        var texels: [UInt8] = [];
        var i = 0;
        while i < 16 {
            texels.append(r);
            texels.append(g);
            texels.append(b);
            texels.append(255);
            i = i + 1;
        }
        device.write(texture: texture, data: texels, layer: layer);
        layer = layer + 1;
    }
    let shaderModule = device.createShaderModule(wgsl: cubeShader);
    let pipeline = device.createRenderPipeline(shader: shaderModule, targets: [ColorTarget(format: colorFormat)]);
    let cubeView = texture.createView(range: ViewRange.cube());
    let sampler = device.createSampler(filter: .Nearest);
    let group = device.createBindGroup(layout: pipeline.bindGroupLayout(0), entries: [cubeView.binding(0), sampler.binding(1)]);
    let color = target(device);
    let view = color.createView();
    let encoder = device.createCommandEncoder();
    encoder.renderPass(color: [view.colorAttachment()]) { (pass) in
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, group);
        pass.draw(vertices: 3);
    };
    device.submit(encoder.finish());
    let bytes = try device.read(texture: color);
    let px = pixel(bytes, 2, 8);
    let py = pixel(bytes, 8, 8);
    let nz = pixel(bytes, 14, 8);
    .Ok(report("cube map views", isColor(px, 255, 0, 0) and isColor(py, 0, 255, 0) and isColor(nz, 0, 0, 255), "+X \(show(px)), +Y \(show(py)), -Z \(show(nz))"))
}

func run() -> Int64 throws WgpuError {
    let instance = Instance();
    let adapter = try instance.requestAdapter();
    let info = adapter.info;
    println("adapter: \(info.name)");
    var features: [Feature] = [];
    if adapter.supports(.TimestampQuery) {
        features.append(.TimestampQuery);
    }
    let pushSize: Int64 = if adapter.supports(.PushConstants) { 48 } else { 0 };
    let device = try adapter.requestDevice(label: "features", features: features, maxPushConstantSize: pushSize);
    let limits = device.limits;
    println("limits: 2D textures up to \(limits.maxTextureDimension2D), \(limits.maxBindGroups) bind groups, uniform offsets aligned to \(limits.minUniformBufferOffsetAlignment)");

    let shader = device.createShaderModule(wgsl: instanceShader, label: "quads");
    var failed: Int64 = 0;
    failed = failed + (try checkMultisampling(device, shader));
    failed = failed + (try checkMipmaps(device));
    failed = failed + (try checkLayersAndCopies(device));
    failed = failed + (try checkComparisonSampler(device, shader));
    failed = failed + (try checkStencilAndWriteMask(device, shader));
    failed = failed + (try checkBlending(device, shader));
    failed = failed + (try checkLayoutsAndDynamicOffsets(device));
    failed = failed + (try checkDrawCalls(device, shader));
    failed = failed + (try checkRenderBundle(device, shader));
    failed = failed + (try checkOcclusionQueries(device, shader));
    failed = failed + (try checkTimestamps(device));
    failed = failed + (try checkPushConstants(device));
    failed = failed + (try checkIndirectDispatch(device));
    failed = failed + (try checkCubeMap(device));
    println("\(failed) failed");
    .Ok(failed)
}

@main
func main() -> Int64 {
    match run() {
        .Ok(failed) => failed,
        .Err(e) => {
            println("wgpu error: \(e.message)");
            100
        }
    }
}
