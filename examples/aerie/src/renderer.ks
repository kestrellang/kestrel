module Aerie

import wgpu.(Instance, Device, Surface, Buffer, BindGroup, BindGroupLayout, RenderPipeline, SurfaceSource, Texture, TextureView, Sampler, TextureFormat, WgpuError, ColorTarget, VertexBufferLayout, Color, DepthTest, BindingLayout, ViewRange)
import sdl.(NativeWindow)
import linalg.(Vec3, Vec4, Mat4, radians)
import images.(Image, Rgba8)

let depthFormat: TextureFormat = .Depth24Plus;
let samples: Int64 = 4;
let tileSize: Int64 = 16;
let skyColor: Vec3 = Vec3(x: 0.70, y: 0.78, z: 0.90);

/// What the shader's `Uniforms` holds, laid out the same way.
struct Uniforms: FFISafe {
    var viewProj: Mat4
    var sunDir: Vec4
    var camera: Vec4
    var fog: Vec4
}

let uniformsSize: Int64 = 112;

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

func surfaceSource(window: NativeWindow) -> SurfaceSource {
    match window {
        .Xlib(display, id) => .Xlib(display, id),
        .Wayland(display, surface) => .Wayland(display, surface),
        .MetalLayer(layer) => .MetalLayer(layer),
        .Windows(hinstance, hwnd) => .Windows(hinstance, hwnd),
        .Unsupported => fatalError("this platform's windows aren't supported")
    }
}

/// The block atlas: `assets/blocks.png` is a column of 16x16 tiles, and
/// each becomes one layer of a mipmapped 2D array texture, so tiles never
/// bleed into each other at a distance.
func loadAtlas(device: Device, path: String) -> Texture {
    let image = match Image[Rgba8].load(path: path) {
        .Ok(i) => i,
        .Err(e) => fatalError("can't load the block atlas \(path): \(e.message)")
    };
    let layers = image.height / tileSize;
    let texture = device.createTexture(
        width: tileSize,
        height: tileSize,
        format: .RGBA8UnormSrgb,
        usage: [.TextureBinding, .CopyDst, .RenderAttachment],
        layers: layers,
        mipLevels: Texture.fullMipChain(width: tileSize, height: tileSize),
        label: "block atlas"
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
    var device: Device
    var surface: Surface
    var format: TextureFormat
    var width: Int64
    var height: Int64
    var depth: Texture
    var color: Texture
    var pipeline: RenderPipeline
    var bindGroup: BindGroup
    var chunkLayout: BindGroupLayout
    var uniformBuffer: Buffer
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

    static func create(window: NativeWindow, width width: Int64, height height: Int64, atlasPath atlasPath: String) -> Renderer throws WgpuError {
        let instance = Instance();
        let surface = instance.createSurface(surfaceSource(window));
        let adapter = try instance.requestAdapter(compatibleWith: surface);
        println("adapter: \(adapter.info.name)");
        let device = try adapter.requestDevice(label: "aerie");
        let format = surface.preferredFormat(adapter);
        surface.configure(device: device, format: format, width: width, height: height);

        let atlas = loadAtlas(device, atlasPath);
        let atlasView = atlas.createView(range: ViewRange(dimension: .Some(.D2Array)), label: "block atlas");
        // Nearest within a mip level keeps the pixel-art edges; linear
        // between levels keeps distant ground from sparkling.
        let atlasSampler = device.createSampler(filter: .Nearest, addressMode: .Repeat, mipmapFilter: .Some(.Linear), label: "block atlas");
        let uniformBuffer = device.createBuffer(size: uniformsSize, usage: [.Uniform, .CopyDst], label: "uniforms");

        let layout = device.createPipelineLayout(groups: [[
            BindingLayout.uniformBuffer(0, visibility: [.Vertex, .Fragment]),
            BindingLayout.texture(1, visibility: [.Fragment], dimension: .D2Array),
            BindingLayout.sampler(2, visibility: [.Fragment])
        ], [
            BindingLayout.uniformBuffer(0, visibility: [.Vertex])
        ]], label: "blocks");
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
        let bindGroup = device.createBindGroup(
            layout: layout.bindGroupLayout(0),
            entries: [uniformBuffer.binding(0), atlasView.binding(1), atlasSampler.binding(2)]
        );
        let depth = Renderer.makeDepth(device, width, height);
        let color = Renderer.makeColor(device, format, width, height);
        .Ok(Renderer(
            device: device, surface: surface, format: format, width: width, height: height,
            depth: depth, color: color, pipeline: pipeline, bindGroup: bindGroup,
            chunkLayout: layout.bindGroupLayout(1), uniformBuffer: uniformBuffer,
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
    mutating func upload(cx: Int64, cy: Int64, cz: Int64, faces: [FaceInstance]) {
        let key = chunkKey(cx, cy, cz);
        self.remove(where: { (k: Int64, x: Int64, z: Int64) in k == key });
        if faces.count == 0 {
            return;
        }
        let size = Float32(from: chunkSize);
        let origin = self.device.createBuffer(
            data: [Vec4(x: Float32(from: cx) * size, y: Float32(from: cy) * size, z: Float32(from: cz) * size, w: 0.0)],
            usage: [.Uniform],
            label: "chunk origin"
        );
        let group = self.device.createBindGroup(layout: self.chunkLayout, entries: [origin.binding(0)]);
        let buffer = self.device.createBuffer(data: faces, usage: [.Vertex], label: "chunk faces");
        self.meshes.append(RcBox(ChunkMesh(key: key, cx: cx, cy: cy, cz: cz, faces: buffer, faceCount: faces.count, origin: origin, group: group)));
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

    /// Draw one frame as seen through `view`, lit by a sun shining along
    /// `sun`.
    mutating func draw(view: Mat4, eye eye: Vec3, sun sun: Vec3) -> () throws WgpuError {
        let aspect = Float32(from: self.width) / Float32(from: self.height);
        let projection = Mat4.perspective(fovY: radians(70.0), aspect: aspect, near: 0.1, far: self.viewDistance + 64.0);
        let viewProj = projection * view;
        self.device.write(self.uniformBuffer, value: Uniforms(
            viewProj: viewProj,
            sunDir: sun.extended(w: 0.0),
            camera: eye.extended(w: 1.0),
            fog: skyColor.extended(w: self.viewDistance)
        ));
        let planes = frustumPlanes(viewProj);

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
        let sky = Color(r: Float64(from: skyColor.x), g: Float64(from: skyColor.y), b: Float64(from: skyColor.z));
        // Frustum-cull before the pass: only chunks the camera can see.
        let size = Float32(from: chunkSize);
        var visible: [Int64] = [];
        var i: Int64 = 0;
        while i < self.meshes.count {
            let m = &self.meshes(i).value;
            let low = Vec3(x: Float32(from: m.cx) * size, y: Float32(from: m.cy) * size, z: Float32(from: m.cz) * size);
            if boxVisible(planes, low, low + Vec3(splat: size)) {
                visible.append(i);
            }
            i = i + 1;
        }
        let encoder = self.device.createCommandEncoder();
        encoder.renderPass(color: [colorView.colorAttachment(resolvingInto: frameView, clear: .Some(sky))], depth: depthView.depthAttachment()) { (pass) in
            pass.setPipeline(self.pipeline);
            pass.setBindGroup(0, self.bindGroup);
            // No `let m = &...` here: a ref binding inside a closure trips
            // an OSSA bug (see binding_element_inside_closure.ks).
            for index in visible {
                pass.setBindGroup(1, self.meshes(index).value.group);
                pass.setVertexBuffer(0, self.meshes(index).value.faces);
                pass.draw(vertices: 6, instances: self.meshes(index).value.faceCount);
            }
        };
        let drawn = visible.count;
        self.drawnChunks = drawn;
        self.device.submit(encoder.finish());
        self.surface.present();
        .Ok(())
    }
}
