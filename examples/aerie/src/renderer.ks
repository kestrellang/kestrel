module Aerie

import wgpu.(Instance, Device, Surface, Buffer, BindGroup, BindGroupLayout, RenderPipeline, SurfaceSource, Texture, TextureView, Sampler, TextureFormat, WgpuError, ColorTarget, VertexBufferLayout, Color, DepthTest, BindingLayout, ViewRange)
import sdl.(NativeWindow)
import linalg.(Vec3, Vec4, Mat4, radians)
import images.(Image, Rgba8)

let depthFormat: TextureFormat = .Depth24Plus;
let samples: Int64 = 4;
let tileSize: Int64 = 16;

/// What the shader's `Uniforms` holds, laid out the same way.
struct Uniforms: FFISafe {
    var viewProj: Mat4
    var lightViewProj: Mat4
    var invViewProj: Mat4
    var sunDir: Vec4
    var camera: Vec4
    var fog: Vec4
    var sky: Vec4
    var zenith: Vec4
    var sunColor: Vec4
}

let uniformsSize: Int64 = 288;
let shadowFormat: TextureFormat = .Depth32Float;
let shadowSize: Int64 = 2048;
/// Half the width of the square around the player that casts shadows.
let shadowReach: Float32 = 72.0;

/// The light at one time of day: where the sun is and how strong, the
/// sky's colors at the horizon and overhead, the sunlight's color, the
/// ambient light from the sky, and how much it is night (for stars).
struct Daylight {
    var sun: Vec3
    var strength: Float32
    var horizon: Vec3
    var zenith: Vec3
    var sunColor: Vec3
    var ambient: Vec3
    var night: Float32
    /// How much of the lairs' crimson gloom is mixed in, 0 to 1.
    var gloom: Float32
}

/// `a` moved toward `b` by `t`, 0 to 1.
func blend(a: Vec3, b: Vec3, t: Float32) -> Vec3 {
    a + (b - a) * t
}

/// `phase` runs 0 to 1 over a day: 0 is sunrise, 0.25 noon, 0.5 sunset,
/// 0.75 midnight. The palette is soft pastel: a pale rose-cream horizon
/// under baby blue by day, apricot at dusk, deep indigo at night. Over the
/// lairs, `gloom` (0 to 1) turns it smoky crimson under a red sun.
func daylight(phase: Float32, gloom gloom: Float32) -> Daylight {
    let angle = phase * Float32.pi * 2.0;
    // Up is +y; the sun crosses the sky from east to west, tilted south.
    let up = angle.sin();
    // The sun moves in steps of a fifth of a degree, about one every 0.3 s
    // of play: each new direction re-rasterizes every shadow edge, so a
    // sun that turned every frame would make shadows crawl.
    let steps: Float32 = 1800.0;
    let stepped = (phase * steps).floor() / steps * Float32.pi * 2.0;
    let sun = Vec3(x: -stepped.cos(), y: -stepped.sin(), z: 0.35).normalized;
    let day = (up * 3.0 + 0.3).clamp(0.0, 1.0);
    // Dusk warms everything just as the sun nears the horizon.
    let dusk = (1.0 - (up * 4.0).abs()).clamp(0.0, 1.0) * (up * 4.0 + 1.5).clamp(0.0, 1.0);
    let horizon = blend(blend(Vec3(x: 0.16, y: 0.15, z: 0.32), Vec3(x: 0.93, y: 0.89, z: 0.92), day), Vec3(x: 1.00, y: 0.74, z: 0.60), dusk * 0.7);
    let zenith = blend(blend(Vec3(x: 0.04, y: 0.04, z: 0.15), Vec3(x: 0.52, y: 0.70, z: 0.96), day), Vec3(x: 0.46, y: 0.48, z: 0.80), dusk * 0.6);
    let sunColor = blend(Vec3(x: 1.00, y: 0.92, z: 0.90), Vec3(x: 1.00, y: 0.70, z: 0.52), dusk);
    // Cool sky light, so shadows read blue-grey.
    let ambient = blend(Vec3(x: 0.12, y: 0.13, z: 0.26), Vec3(x: 0.54, y: 0.54, z: 0.64), day);
    // The lairs' palette, mixed in by `gloom`.
    let g = gloom * 0.9;
    let lairHorizon = blend(Vec3(x: 0.12, y: 0.03, z: 0.06), Vec3(x: 0.40, y: 0.11, z: 0.12), day);
    let lairZenith = blend(Vec3(x: 0.03, y: 0.01, z: 0.05), Vec3(x: 0.10, y: 0.04, z: 0.10), day);
    let lairAmbient = blend(Vec3(x: 0.14, y: 0.06, z: 0.10), Vec3(x: 0.36, y: 0.22, z: 0.28), day);
    Daylight(
        sun: sun, strength: day,
        horizon: blend(horizon, lairHorizon, g), zenith: blend(zenith, lairZenith, g),
        sunColor: blend(sunColor, Vec3(x: 1.0, y: 0.50, z: 0.32), g), ambient: blend(ambient, lairAmbient, g),
        night: 1.0 - day, gloom: gloom
    )
}

/// The sun's view: an orthographic box around `center`, looking along
/// `sun`. The box moves in whole shadow texels of the sun's own view, so
/// shadows don't shimmer as the player walks: snapping in world axes
/// wouldn't line up with the map's texels, since the sun looks at an angle.
func lightViewProjection(sun: Vec3, center: Vec3) -> Mat4 {
    let texel = shadowReach * 2.0 / Float32(from: shadowSize);
    // A view that only turns to face along `sun`; `center` in its axes.
    let f = sun.normalized;
    let view = Mat4.lookAt(eye: Vec3(x: 0.0, y: 0.0, z: 0.0) - f, target: Vec3(x: 0.0, y: 0.0, z: 0.0), up: Vec3.unitY);
    let side = f.cross(Vec3.unitY).normalized;
    let up = side.cross(f);
    let x = (side.dot(center) / texel).floor() * texel;
    let y = (up.dot(center) / texel).floor() * texel;
    // Distance along the view, which looks down its -z: the eye sits one
    // unit behind the origin.
    let depth = f.dot(center) + 1.0;
    Mat4.orthographic(left: x - shadowReach, right: x + shadowReach, bottom: y - shadowReach, top: y + shadowReach, near: depth - 150.0, far: depth + 150.0) * view
}

/// One overlay rectangle: `rect` is (x, y, width, height) in pixels from
/// the top left. `tile.x` is a layer to draw tinted by `color`, or negative
/// for a plain `color` fill; `tile.y` picks the block atlas (0) or the
/// interface atlas (1); `tile.z` to `tile.w` is the span of the layer's
/// width to show, so a sprite can be drawn in stretched slices.
struct OverlayQuad: FFISafe {
    var rect: Vec4
    var color: Vec4
    var tile: Vec4
}

/// Text is one quad per run of lit dots, so a few lines take a thousand or more.
let maxOverlayQuads: Int64 = 4096;
let overlayQuadSize: Int64 = 48;

/// A chunk's faces on the GPU, with its own small uniform holding where the
/// chunk sits in the world (bind group 1). A bind group per chunk, rather
/// than push constants, keeps this portable to WebGPU in a browser.
struct ChunkMesh: not Copyable {
    var key: Int64
    var cx: Int64
    var cy: Int64
    var cz: Int64
    var faces: Buffer
    var faceCount: Int64
    /// Translucent faces; a one-face placeholder buffer when there are none,
    /// since a buffer can't be empty.
    var glass: Buffer
    var glassCount: Int64
    var origin: Buffer
    var group: BindGroup
}

/// The six planes of a view-projection's frustum, each as (a, b, c, d)
/// with the inside where ax + by + cz + d >= 0.
func frustumPlanes(m: Mat4) -> [Vec4] {
    // Row i of the column-major matrix.
    let r0 = Vec4(x: m.c0.x, y: m.c1.x, z: m.c2.x, w: m.c3.x);
    let r1 = Vec4(x: m.c0.y, y: m.c1.y, z: m.c2.y, w: m.c3.y);
    let r2 = Vec4(x: m.c0.z, y: m.c1.z, z: m.c2.z, w: m.c3.z);
    let r3 = Vec4(x: m.c0.w, y: m.c1.w, z: m.c2.w, w: m.c3.w);
    // WebGPU clip space has 0 <= z <= w.
    [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2]
}

/// Whether any of the box from `low` to `high` is inside every plane.
func boxVisible(planes: [Vec4], low: Vec3, high: Vec3) -> Bool {
    for p in planes {
        // The box corner furthest along the plane's normal.
        let x = if p.x >= 0.0 { high.x } else { low.x };
        let y = if p.y >= 0.0 { high.y } else { low.y };
        let z = if p.z >= 0.0 { high.z } else { low.z };
        if p.x * x + p.y * y + p.z * z + p.w < 0.0 {
            return false;
        }
    }
    true
}

/// An atlas image that is a column of `size`-pixel square tiles, as a
/// mipmapped 2D array texture with one layer per tile, so tiles never
/// bleed into each other at a distance. Used for the block atlas
/// (`assets/blocks.png`, 16x16) and the interface atlas (`assets/ui.png`).
func loadAtlas(device: Device, path: String, size tileSize: Int64) -> Texture {
    let image = match Image[Rgba8].load(path: path) {
        .Ok(i) => i,
        .Err(e) => fatalError("can't load the atlas \(path): \(e.message)")
    };
    let layers = image.height / tileSize;
    let texture = device.createTexture(
        width: tileSize,
        height: tileSize,
        format: .RGBA8UnormSrgb,
        usage: [.TextureBinding, .CopyDst, .RenderAttachment],
        layers: layers,
        mipLevels: Texture.fullMipChain(width: tileSize, height: tileSize),
        label: path
    );
    let tileBytes = tileSize * tileSize * 4;
    var layer: Int64 = 0;
    while layer < layers {
        var tile: [UInt8] = [];
        tile.reserveCapacity(tileBytes);
        var i = layer * tileBytes;
        while i < (layer + 1) * tileBytes {
            tile.append(image.bytes(i));
            i = i + 1;
        }
        device.write(texture: texture, data: tile, layer: layer);
        layer = layer + 1;
    }
    device.generateMipmaps(texture);
    texture
}

/// Everything on the GPU: the window surface, the block pipeline and the
/// buffers it reads. The game hands it a camera and faces; it knows nothing
/// about blocks itself.
struct Renderer: not Copyable {
    /// Kept for the renderer's lifetime: a browser's WebGPU stops serving
    /// the surface once its instance is released.
    var instance: Instance
    var device: Device
    var surface: Surface
    var format: TextureFormat
    var width: Int64
    var height: Int64
    var depth: Texture
    var color: Texture
    var pipeline: RenderPipeline
    var glassPipeline: RenderPipeline
    /// The sky, drawn first behind everything: gradient, sun, stars, clouds.
    var skyPipeline: RenderPipeline
    var bindGroup: BindGroup
    var shadowMap: Texture
    var shadowPipeline: RenderPipeline
    var shadowGroup: BindGroup
    var chunkLayout: BindGroupLayout
    var uniformBuffer: Buffer
    /// Boxes at any angle: birds, and the cracks on a block being broken.
    var modelPipeline: RenderPipeline
    var modelShadowPipeline: RenderPipeline
    var modelBuffer: Buffer
    var overlayPipeline: RenderPipeline
    var overlayGroup: BindGroup
    var screenBuffer: Buffer
    var overlayBuffer: Buffer
    /// Boxed, since arrays hold only copyable values; dropping the last box
    /// frees the chunk's GPU buffers.
    var meshes: [RcBox[ChunkMesh]]
    /// How far, in blocks, chunks are drawn; fog closes in before it.
    var viewDistance: Float32
    /// How many chunk meshes passed frustum culling last frame.
    var drawnChunks: Int64

    /// The multisampled depth buffer for the current size.
    static func makeDepth(device: Device, width: Int64, height: Int64) -> Texture {
        device.createTexture(width: width, height: height, format: depthFormat, usage: [.RenderAttachment], sampleCount: samples, label: "depth")
    }

    /// The multisampled color buffer, resolved into each frame.
    static func makeColor(device: Device, format: TextureFormat, width: Int64, height: Int64) -> Texture {
        device.createTexture(width: width, height: height, format: format, usage: [.RenderAttachment], sampleCount: samples, label: "color")
    }

    static func create(window: NativeWindow, width width: Int64, height height: Int64) -> Renderer throws WgpuError {
        let instance = Instance();
        let surface = instance.createSurface(surfaceSource(window));
        let adapter = try instance.requestAdapter(compatibleWith: surface);
        println("adapter: \(adapter.info.name)");
        let device = try adapter.requestDevice(label: "aerie");
        let format = surface.preferredFormat(adapter);
        surface.configure(device: device, format: format, width: width, height: height);

        let atlas = loadAtlas(device, "assets/blocks.png", size: tileSize);
        let ui = loadAtlas(device, "assets/ui.png", size: uiCellSize);
        let uiView = ui.createView(range: ViewRange(dimension: .Some(.D2Array)), label: "interface atlas");
        // Smooth: the interface atlas holds font glyphs and drawn sprites.
        let uiSampler = device.createSampler(filter: .Linear, mipmapFilter: .Some(.Linear), label: "interface atlas");
        let atlasView = atlas.createView(range: ViewRange(dimension: .Some(.D2Array)), label: "block atlas");
        // Nearest within a mip level keeps the pixel-art edges; linear
        // between levels keeps distant ground from sparkling.
        let atlasSampler = device.createSampler(filter: .Nearest, addressMode: .Repeat, mipmapFilter: .Some(.Linear), label: "block atlas");
        let shadowMap = device.createTexture(width: shadowSize, height: shadowSize, format: shadowFormat, usage: [.RenderAttachment, .TextureBinding], label: "shadow map");
        let shadowView = shadowMap.createView();
        let shadowSampler = device.createSampler(filter: .Linear, compare: .Some(.LessEqual), label: "shadow");
        let uniformBuffer = device.createBuffer(size: uniformsSize, usage: [.Uniform, .CopyDst], label: "uniforms");

        let layout = device.createPipelineLayout(groups: [[
            BindingLayout.uniformBuffer(0, visibility: [.Vertex, .Fragment]),
            BindingLayout.texture(1, visibility: [.Fragment], dimension: .D2Array),
            BindingLayout.sampler(2, visibility: [.Fragment]),
            BindingLayout.texture(3, visibility: [.Fragment], sampleType: .Depth),
            BindingLayout.sampler(4, visibility: [.Fragment], kind: .Comparison)
        ], [
            BindingLayout.uniformBuffer(0, visibility: [.Vertex, .Fragment])
        ]], label: "blocks");
        // The shadow pass can't bind the shadow map it draws into.
        let shadowLayout = device.createPipelineLayout(groups: [[
            BindingLayout.uniformBuffer(0, visibility: [.Vertex])
        ], [
            BindingLayout.uniformBuffer(0, visibility: [.Vertex, .Fragment])
        ]], label: "shadow");
        let shaderModule = device.createShaderModule(wgsl: blockShader(), label: "blocks");
        let pipeline = device.createRenderPipeline(
            shader: shaderModule,
            targets: [ColorTarget(format: format)],
            buffers: [VertexBufferLayout(packed: [.Uint32, .Uint32], stepMode: .Instance)],
            cullMode: .Back,
            depth: .Some(DepthTest(format: depthFormat)),
            sampleCount: samples,
            layout: .Some(layout.share()),
            label: "blocks"
        );
        // Glass blends over the opaque world: tested against its depth but
        // not written, so glass behind glass still shows.
        let glassPipeline = device.createRenderPipeline(
            shader: shaderModule,
            targets: [ColorTarget(format: format, blend: .AlphaBlending)],
            buffers: [VertexBufferLayout(packed: [.Uint32, .Uint32], stepMode: .Instance)],
            fragmentEntry: "fs_glass",
            cullMode: .Back,
            depth: .Some(DepthTest(format: depthFormat, write: false)),
            sampleCount: samples,
            layout: .Some(layout.share()),
            label: "glass"
        );
        // The sky reads only group 0; an identical layout lets it share
        // the blocks' bind group.
        let skyLayout = device.createPipelineLayout(groups: [[
            BindingLayout.uniformBuffer(0, visibility: [.Vertex, .Fragment]),
            BindingLayout.texture(1, visibility: [.Fragment], dimension: .D2Array),
            BindingLayout.sampler(2, visibility: [.Fragment]),
            BindingLayout.texture(3, visibility: [.Fragment], sampleType: .Depth),
            BindingLayout.sampler(4, visibility: [.Fragment], kind: .Comparison)
        ]], label: "sky");
        let skyPipeline = device.createRenderPipeline(
            shader: shaderModule,
            targets: [ColorTarget(format: format)],
            vertexEntry: "vs_sky",
            fragmentEntry: "fs_sky",
            depth: .Some(DepthTest(format: depthFormat, compare: .Always, write: false)),
            sampleCount: samples,
            layout: .Some(skyLayout.share()),
            label: "sky"
        );
        let bindGroup = device.createBindGroup(
            layout: layout.bindGroupLayout(0),
            entries: [uniformBuffer.binding(0), atlasView.binding(1), atlasSampler.binding(2), shadowView.binding(3), shadowSampler.binding(4)]
        );
        // Depth only; the bias keeps lit faces from shadowing themselves.
        let shadowPipeline = device.createRenderPipeline(
            shader: shaderModule,
            targets: [],
            buffers: [VertexBufferLayout(packed: [.Uint32, .Uint32], stepMode: .Instance)],
            vertexEntry: "vs_shadow",
            depth: .Some(DepthTest(format: shadowFormat, depthBias: 2, depthBiasSlopeScale: 2.0)),
            layout: .Some(shadowLayout.share()),
            label: "shadow"
        );
        let shadowGroup = device.createBindGroup(layout: shadowLayout.bindGroupLayout(0), entries: [uniformBuffer.binding(0)]);
        // Models read only group 0, like the sky. Both faces are drawn:
        // a mirrored wing turns its boxes inside out.
        let modelBuffers = [VertexBufferLayout(packed: [.Float32x4, .Float32x4, .Float32x4, .Float32x4], stepMode: .Instance)];
        let modelPipeline = device.createRenderPipeline(
            shader: shaderModule,
            targets: [ColorTarget(format: format)],
            buffers: modelBuffers,
            vertexEntry: "vs_model",
            fragmentEntry: "fs_model",
            depth: .Some(DepthTest(format: depthFormat)),
            sampleCount: samples,
            layout: .Some(skyLayout.share()),
            label: "models"
        );
        let modelShadowLayout = device.createPipelineLayout(groups: [[
            BindingLayout.uniformBuffer(0, visibility: [.Vertex])
        ]], label: "model shadow");
        let modelShadowPipeline = device.createRenderPipeline(
            shader: shaderModule,
            targets: [],
            buffers: modelBuffers,
            vertexEntry: "vs_model_shadow",
            depth: .Some(DepthTest(format: shadowFormat, depthBias: 2, depthBiasSlopeScale: 2.0)),
            layout: .Some(modelShadowLayout.share()),
            label: "model shadow"
        );
        let modelBuffer = device.createBuffer(size: maxModelBoxes * modelBoxSize, usage: [.Vertex, .CopyDst], label: "models");
        // The overlay draws in the same pass, over everything: it keeps the
        // pass's depth format but never tests or writes depth.
        let overlayLayout = device.createPipelineLayout(groups: [[
            BindingLayout.uniformBuffer(0, visibility: [.Vertex]),
            BindingLayout.texture(1, visibility: [.Fragment], dimension: .D2Array),
            BindingLayout.sampler(2, visibility: [.Fragment]),
            BindingLayout.texture(3, visibility: [.Fragment], dimension: .D2Array),
            BindingLayout.sampler(4, visibility: [.Fragment])
        ]], label: "overlay");
        let overlayModule = device.createShaderModule(wgsl: overlayShader, label: "overlay");
        let overlayPipeline = device.createRenderPipeline(
            shader: overlayModule,
            targets: [ColorTarget(format: format, blend: .AlphaBlending)],
            buffers: [VertexBufferLayout(packed: [.Float32x4, .Float32x4, .Float32x4], stepMode: .Instance)],
            depth: .Some(DepthTest(format: depthFormat, compare: .Always, write: false)),
            sampleCount: samples,
            layout: .Some(overlayLayout.share()),
            label: "overlay"
        );
        let screenBuffer = device.createBuffer(size: 16, usage: [.Uniform, .CopyDst], label: "screen");
        let overlayGroup = device.createBindGroup(
            layout: overlayLayout.bindGroupLayout(0),
            entries: [screenBuffer.binding(0), atlasView.binding(1), atlasSampler.binding(2), uiView.binding(3), uiSampler.binding(4)]
        );
        let overlayBuffer = device.createBuffer(size: maxOverlayQuads * overlayQuadSize, usage: [.Vertex, .CopyDst], label: "overlay");
        let depth = Renderer.makeDepth(device, width, height);
        let color = Renderer.makeColor(device, format, width, height);
        .Ok(Renderer(
            instance: instance, device: device, surface: surface, format: format, width: width, height: height,
            depth: depth, color: color, pipeline: pipeline, glassPipeline: glassPipeline, skyPipeline: skyPipeline, bindGroup: bindGroup,
            shadowMap: shadowMap, shadowPipeline: shadowPipeline, shadowGroup: shadowGroup,
            chunkLayout: layout.bindGroupLayout(1), uniformBuffer: uniformBuffer,
            modelPipeline: modelPipeline, modelShadowPipeline: modelShadowPipeline, modelBuffer: modelBuffer,
            overlayPipeline: overlayPipeline, overlayGroup: overlayGroup, screenBuffer: screenBuffer, overlayBuffer: overlayBuffer,
            meshes: [], viewDistance: 160.0, drawnChunks: 0
        ))
    }

    /// Match a new window size.
    mutating func resize(width: Int64, height: Int64) {
        self.width = width;
        self.height = height;
        self.surface.configure(device: self.device, format: self.format, width: width, height: height);
        self.depth = Renderer.makeDepth(self.device, width, height);
        self.color = Renderer.makeColor(self.device, self.format, width, height);
    }

    /// Draw `faces` for chunk (cx, cy, cz), replacing any it had.
    mutating func upload(cx: Int64, cy: Int64, cz: Int64, faces: ChunkFaces, lights: [Vec4]) {
        let key = chunkKey(cx, cy, cz);
        self.remove(where: { (k: Int64, x: Int64, z: Int64) in k == key });
        if faces.count == 0 {
            return;
        }
        let size = Float32(from: chunkSize);
        // The shader's `ChunkData`: the origin with the light count in w,
        // then a fixed-size array of light positions, each with its color
        // (`Block.glow`) in w.
        var data = [Vec4(x: Float32(from: cx) * size, y: Float32(from: cy) * size, z: Float32(from: cz) * size, w: Float32(from: lights.count))];
        for light in lights {
            data.append(light);
        }
        while data.count < maxLights + 1 {
            data.append(Vec4(x: 0.0, y: 0.0, z: 0.0, w: 0.0));
        }
        let origin = self.device.createBuffer(data: data, usage: [.Uniform], label: "chunk data");
        let group = self.device.createBindGroup(layout: self.chunkLayout, entries: [origin.binding(0)]);
        let opaque = if faces.opaque.count > 0 { faces.opaque } else { [FaceInstance(packed: 0, shade: 0)] };
        let glass = if faces.glass.count > 0 { faces.glass } else { [FaceInstance(packed: 0, shade: 0)] };
        self.meshes.append(RcBox(ChunkMesh(
            key: key, cx: cx, cy: cy, cz: cz,
            faces: self.device.createBuffer(data: opaque, usage: [.Vertex], label: "chunk faces"),
            faceCount: faces.opaque.count,
            glass: self.device.createBuffer(data: glass, usage: [.Vertex], label: "chunk glass"),
            glassCount: faces.glass.count,
            origin: origin, group: group
        )));
    }

    /// Drop the meshes `test` picks, given each one's key and chunk x and z.
    mutating func remove(where test: (Int64, Int64, Int64) -> Bool) {
        var i: Int64 = 0;
        while i < self.meshes.count {
            let m = &self.meshes(i).value;
            if test(m.key, m.cx, m.cz) {
                let _ = self.meshes.remove(at: i);
            } else {
                i = i + 1;
            }
        }
    }

    /// Draw one frame as seen through `view`, in the light of `light`, at
    /// `time` seconds (which drifts the clouds). The first `casters` of
    /// `models` cast shadows.
    mutating func draw(view: Mat4, eye eye: Vec3, light light: Daylight, time time: Float32, models models: [ModelBox], casters modelCasters: Int64, overlay overlay: [OverlayQuad]) -> () throws WgpuError {
        let aspect = Float32(from: self.width) / Float32(from: self.height);
        let projection = Mat4.perspective(fovY: radians(70.0), aspect: aspect, near: 0.1, far: self.viewDistance + 64.0);
        let viewProj = projection * view;
        let lightViewProj = lightViewProjection(light.sun, eye);
        self.device.write(self.uniformBuffer, value: Uniforms(
            viewProj: viewProj,
            lightViewProj: lightViewProj,
            invViewProj: viewProj.inverse ?? Mat4.identity,
            sunDir: light.sun.extended(w: light.strength),
            camera: eye.extended(w: time),
            fog: light.horizon.extended(w: self.viewDistance),
            sky: light.ambient.extended(w: light.night),
            zenith: light.zenith.extended(w: 0.0),
            sunColor: light.sunColor.extended(w: light.gloom)
        ));
        let shadowPlanes = frustumPlanes(lightViewProj);
        let planes = frustumPlanes(viewProj);
        self.device.write(self.screenBuffer, value: Vec4(x: Float32(from: self.width), y: Float32(from: self.height), z: 0.0, w: 0.0));
        var quads = overlay;
        while quads.count > maxOverlayQuads {
            let _ = quads.pop();
        }
        let quadCount = quads.count;
        if quadCount > 0 {
            self.device.write(self.overlayBuffer, data: quads);
        }
        var boxes = models;
        while boxes.count > maxModelBoxes {
            let _ = boxes.pop();
        }
        let boxCount = boxes.count;
        let shadowBoxes = modelCasters.clamp(0, boxCount);
        if boxCount > 0 {
            self.device.write(self.modelBuffer, data: boxes);
        }

        let frame = match self.surface.currentTexture() {
            .Ok(f) => f,
            .Err(.SurfaceOutdated(_)) => {
                self.surface.configure(device: self.device, format: self.format, width: self.width, height: self.height);
                return .Ok(());
            },
            .Err(.SurfaceTimeout(_)) => { return .Ok(()); },
            .Err(e) => { throw e }
        };
        let frameView = frame.texture.createView();
        let colorView = self.color.createView();
        let depthView = self.depth.createView();
        let sky = Color(r: Float64(from: light.horizon.x), g: Float64(from: light.horizon.y), b: Float64(from: light.horizon.z));
        // Frustum-cull before the pass: only chunks the camera can see.
        let size = Float32(from: chunkSize);
        var visible: [Int64] = [];
        var casters: [Int64] = [];
        // Visible chunks with glass, by squared distance to their middle.
        var glassy: [(Float32, Int64)] = [];
        var i: Int64 = 0;
        while i < self.meshes.count {
            let m = &self.meshes(i).value;
            let low = Vec3(x: Float32(from: m.cx) * size, y: Float32(from: m.cy) * size, z: Float32(from: m.cz) * size);
            if boxVisible(planes, low, low + Vec3(splat: size)) {
                visible.append(i);
                if m.glassCount > 0 {
                    glassy.append(((low + Vec3(splat: size * 0.5) - eye).lengthSquared, i));
                }
            }
            if boxVisible(shadowPlanes, low, low + Vec3(splat: size)) {
                casters.append(i);
            }
            i = i + 1;
        }
        // Blending needs back to front: far chunks first. Faces within a
        // chunk aren't sorted, which only shows where glass overlaps glass.
        glassy.sort(by: { (a: (Float32, Int64), b: (Float32, Int64)) in a.0 > b.0 });
        var glassOrder: [Int64] = [];
        for (_, index) in glassy {
            glassOrder.append(index);
        }
        let shadowView = self.shadowMap.createView();
        let encoder = self.device.createCommandEncoder();
        encoder.renderPass(color: [], depth: shadowView.depthAttachment()) { (pass) in
            pass.setPipeline(self.shadowPipeline);
            pass.setBindGroup(0, self.shadowGroup);
            for index in casters {
                pass.setBindGroup(1, self.meshes(index).value.group);
                pass.setVertexBuffer(0, self.meshes(index).value.faces);
                pass.draw(vertices: 6, instances: self.meshes(index).value.faceCount);
            }
            if shadowBoxes > 0 {
                pass.setPipeline(self.modelShadowPipeline);
                pass.setBindGroup(0, self.shadowGroup);
                pass.setVertexBuffer(0, self.modelBuffer);
                pass.draw(vertices: 36, instances: shadowBoxes);
            }
        };
        encoder.renderPass(color: [colorView.colorAttachment(resolvingInto: frameView, clear: .Some(sky))], depth: depthView.depthAttachment()) { (pass) in
            pass.setPipeline(self.skyPipeline);
            pass.setBindGroup(0, self.bindGroup);
            pass.draw(vertices: 3);
            pass.setPipeline(self.pipeline);
            // No `let m = &...` here: a ref binding inside a closure trips
            // an OSSA bug (see binding_element_inside_closure.ks).
            for index in visible {
                pass.setBindGroup(1, self.meshes(index).value.group);
                pass.setVertexBuffer(0, self.meshes(index).value.faces);
                pass.draw(vertices: 6, instances: self.meshes(index).value.faceCount);
            }
            if boxCount > 0 {
                pass.setPipeline(self.modelPipeline);
                pass.setVertexBuffer(0, self.modelBuffer);
                pass.draw(vertices: 36, instances: boxCount);
            }
            pass.setPipeline(self.glassPipeline);
            for index in glassOrder {
                pass.setBindGroup(1, self.meshes(index).value.group);
                pass.setVertexBuffer(0, self.meshes(index).value.glass);
                pass.draw(vertices: 6, instances: self.meshes(index).value.glassCount);
            }
            if quadCount > 0 {
                pass.setPipeline(self.overlayPipeline);
                pass.setBindGroup(0, self.overlayGroup);
                pass.setVertexBuffer(0, self.overlayBuffer);
                pass.draw(vertices: 6, instances: quadCount);
            }
        };
        let drawn = visible.count;
        self.drawnChunks = drawn;
        self.device.submit(encoder.finish());
        self.surface.present();
        .Ok(())
    }
}
