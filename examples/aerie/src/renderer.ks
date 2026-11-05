module Aerie

import wgpu.(Instance, Device, Surface, Buffer, BindGroup, RenderPipeline, SurfaceSource, Texture, TextureView, Sampler, TextureFormat, WgpuError, ColorTarget, VertexBufferLayout, Color, DepthTest, BindingLayout, ViewRange)
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
    var chunkOrigin: Vec4
}

let uniformsSize: Int64 = 128;

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
    var uniformBuffer: Buffer
    var faceBuffer: Buffer
    var faceCount: Int64

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
        let faceBuffer = device.createBuffer(size: 8, usage: [.Vertex, .CopyDst], label: "faces");
        .Ok(Renderer(
            device: device, surface: surface, format: format, width: width, height: height,
            depth: depth, color: color, pipeline: pipeline, bindGroup: bindGroup,
            uniformBuffer: uniformBuffer, faceBuffer: faceBuffer, faceCount: 0
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

    /// Replace the faces to draw.
    mutating func upload(faces: [FaceInstance]) {
        self.faceCount = faces.count;
        self.faceBuffer = self.device.createBuffer(data: faces, usage: [.Vertex], label: "faces");
    }

    /// Draw one frame as seen through `view`, lit by a sun shining along
    /// `sun`.
    mutating func draw(view: Mat4, eye eye: Vec3, sun sun: Vec3) -> () throws WgpuError {
        let aspect = Float32(from: self.width) / Float32(from: self.height);
        let projection = Mat4.perspective(fovY: radians(70.0), aspect: aspect, near: 0.1, far: 400.0);
        self.device.write(self.uniformBuffer, value: Uniforms(
            viewProj: projection * view,
            sunDir: sun.extended(w: 0.0),
            camera: eye.extended(w: 1.0),
            fog: skyColor.extended(w: 0.0035),
            chunkOrigin: Vec4(x: 0.0, y: 0.0, z: 0.0, w: 0.0)
        ));

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
        let encoder = self.device.createCommandEncoder();
        encoder.renderPass(color: [colorView.colorAttachment(resolvingInto: frameView, clear: .Some(sky))], depth: depthView.depthAttachment()) { (pass) in
            pass.setPipeline(self.pipeline);
            pass.setBindGroup(0, self.bindGroup);
            pass.setVertexBuffer(0, self.faceBuffer);
            pass.draw(vertices: 6, instances: self.faceCount);
        };
        self.device.submit(encoder.finish());
        self.surface.present();
        .Ok(())
    }
}
