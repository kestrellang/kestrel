module Aerie

import wgpu.(RenderPassDescriptor, Feature, Instance, Device, Surface, PresentMode, Buffer, BindGroup, RenderPipeline, Texture, TextureView, TextureFormat, WgpuError, ColorTarget, ColorAttachment, Color, DepthTest, BindingLayout, ViewRange, PipelineLayout)
import sdl.(NativeWindow)
import linalg.(Vec3, Vec4, Mat4, radians)

let depthFormat: TextureFormat = .Depth24Plus;
let shadowFormat: TextureFormat = .Depth32Float;
/// The usual vertical field of view, in degrees; it widens with speed.
let baseFieldOfView: Float32 = 70.0;
/// The block atlas's tiles are this many pixels square.
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

extend TextureFormat {
    /// Whether the surface encodes linear color to sRGB as it's written.
    var isSrgb: Bool {
        match self {
            .RGBA8UnormSrgb => true,
            .BGRA8UnormSrgb => true,
            _ => false
        }
    }
}

/// Bind group 0 of the world's shaders: the uniforms, the block atlas and
/// its sampler, and the shadow map and its comparison sampler.
func worldBindings() -> [BindingLayout] {
    [
        BindingLayout.uniformBuffer(0, visibility: [.Vertex, .Fragment]),
        BindingLayout.texture(1, visibility: [.Fragment], dimension: .D2Array),
        BindingLayout.sampler(2, visibility: [.Fragment]),
        BindingLayout.texture(3, visibility: [.Fragment], sampleType: .Depth),
        BindingLayout.sampler(4, visibility: [.Fragment], kind: .Comparison)
    ]
}

/// Bind group 1 of the block and shadow shaders: every chunk's data.
func chunkBindings() -> [BindingLayout] {
    [BindingLayout.storageBuffer(0, visibility: [.Vertex, .Fragment], readOnly: true)]
}

/// Everything on the GPU: the window surface, the frame's passes and the
/// buffers they share. The game hands it a camera, faces, boxes and
/// overlay quads; it knows nothing about blocks itself.
struct Renderer: not Copyable {
    /// Kept for the renderer's lifetime: a browser's WebGPU stops serving
    /// the surface once its instance is released.
    var instance: Instance
    var device: Device
    var surface: Surface
    /// What the main pass draws into.
    var target: DrawTarget
    var graphics: Graphics
    var width: Int64
    var height: Int64
    var depth: Texture
    /// The multisampled color buffer, resolved into each frame; None
    /// without multisampling, when the pass draws straight to the frame.
    var colorView: TextureView?
    /// The sky, drawn first behind everything: gradient, sun, stars, clouds.
    var skyPipeline: RenderPipeline
    /// Bind group 0 of the world, the sky and models.
    var worldGroup: BindGroup
    var shadowMap: Texture
    /// Bind group 0 of the shadow pipelines: the uniforms alone, since the
    /// shadow pass can't read the shadow map it draws into.
    var shadowGroup: BindGroup
    var uniformBuffer: Buffer
    var chunks: ChunkPass
    /// Boxes at any angle: birds, and the cracks on a block being broken.
    var models: ModelPass
    var overlay: OverlayPass
    var icons: IconPass
    /// How far, in blocks, chunks are drawn; fog closes in before it.
    var viewDistance: Float32
    /// The vertical field of view, in degrees.
    var fieldOfView: Float32
    /// How many chunk meshes passed frustum culling last frame, and how
    /// many draw calls they took.
    var drawnChunks: Int64
    var chunkDraws: Int64
    /// GPU time per pass, once `gpuTimer.start` is called.
    var gpuTimer: GpuTimer

    /// The depth buffer for `target` at the current size.
    static func makeDepth(device: Device, target: DrawTarget, width: Int64, height: Int64) -> Texture {
        device.createTexture(width: width, height: height, format: depthFormat, usage: [.RenderAttachment], sampleCount: target.samples, label: "depth")
    }

    /// The multisampled color buffer for `target`, or None when it isn't
    /// multisampled.
    static func makeColor(device: Device, target: DrawTarget, width: Int64, height: Int64) -> TextureView? {
        guard target.samples > 1 else {
            return .None;
        }
        .Some(device.createTexture(width: width, height: height, format: target.format, usage: [.RenderAttachment], sampleCount: target.samples, label: "color").createView())
    }

    static func create(window: NativeWindow, width width: Int64, height height: Int64, graphics graphics: Graphics) -> Renderer throws WgpuError {
        let instance = Instance();
        let surface = instance.createSurface(surfaceSource(window));
        let adapter = try instance.requestAdapter(compatibleWith: surface);
        println("adapter: \(adapter.info.name)");
        // Timestamps cost nothing until a run asks to time the GPU.
        let features = if adapter.supports(.TimestampQuery) { [Feature.TimestampQuery] } else { [] };
        let device = try adapter.requestDevice(label: "aerie", features: features);
        let quality = graphics.quality;
        let target = DrawTarget(format: surface.preferredFormat(adapter), samples: quality.samples);
        surface.configure(device: device, format: target.format, width: width, height: height, presentMode: graphics.presentMode);

        let atlas = Atlas.load(device, "assets/blocks.png", tileSize: tileSize);
        let atlasView = atlas.createView(range: ViewRange(dimension: .Some(.D2Array)), label: "block atlas");
        // Nearest within a mip level keeps the pixel-art edges; linear
        // between levels keeps distant ground from sparkling.
        let atlasSampler = device.createSampler(filter: .Nearest, addressMode: .Repeat, mipmapFilter: .Some(.Linear), label: "block atlas");
        let ui = Atlas.load(device, "assets/ui.png", tileSize: uiCellSize);
        let uiView = ui.createView(range: ViewRange(dimension: .Some(.D2Array)), label: "interface atlas");
        // Smooth: the interface atlas holds font glyphs and drawn sprites.
        let uiSampler = device.createSampler(filter: .Linear, mipmapFilter: .Some(.Linear), label: "interface atlas");
        let shadowMap = device.createTexture(width: quality.shadowSize, height: quality.shadowSize, format: shadowFormat, usage: [.RenderAttachment, .TextureBinding], label: "shadow map");
        let shadowView = shadowMap.createView();
        let shadowSampler = device.createSampler(filter: .Linear, compare: .Some(.LessEqual), label: "shadow");
        let uniformBuffer = device.createBuffer(size: Layout.of[Uniforms]().size, usage: [.Uniform, .CopyDst], label: "uniforms");

        let layout = device.createPipelineLayout(groups: [worldBindings(), chunkBindings()], label: "blocks");
        // The sky and models read only group 0, so they share its bind group.
        let skyLayout = device.createPipelineLayout(groups: [worldBindings()], label: "sky");
        let shadowLayout = device.createPipelineLayout(groups: [[
            BindingLayout.uniformBuffer(0, visibility: [.Vertex])
        ], chunkBindings()], label: "shadow");
        let header = shaderHeader(target, quality: quality);
        let shader = device.createShaderModule(wgsl: loadShader("blocks", header: header), label: "blocks");
        let overlayShader = device.createShaderModule(wgsl: loadShader("overlay", header: header), label: "overlay");
        let skyPipeline = device.createRenderPipeline(
            shader: shader,
            targets: [ColorTarget(format: target.format)],
            vertexEntry: "vs_sky",
            fragmentEntry: "fs_sky",
            depth: .Some(DepthTest(format: depthFormat, compare: .Always, write: false)),
            sampleCount: target.samples,
            layout: .Some(skyLayout.share()),
            label: "sky"
        );
        let worldGroup = device.createBindGroup(
            layout: layout.bindGroupLayout(0),
            entries: [uniformBuffer.binding(0), atlasView.binding(1), atlasSampler.binding(2), shadowView.binding(3), shadowSampler.binding(4)]
        );
        let shadowGroup = device.createBindGroup(layout: shadowLayout.bindGroupLayout(0), entries: [uniformBuffer.binding(0)]);
        let chunks = ChunkPass.create(device, shader, target: target, layout: layout, shadowLayout: shadowLayout);
        let models = ModelPass.create(device, shader, target: target, layout: skyLayout);
        let icons = IconPass.create(device, shader, target: target);
        let overlay = OverlayPass.create(device, overlayShader, target: target, blocks: atlasView, blockSampler: atlasSampler, ui: uiView, uiSampler: uiSampler);
        let depth = Renderer.makeDepth(device, target, width, height);
        let colorView = Renderer.makeColor(device, target, width, height);
        .Ok(Renderer(
            instance: instance, device: device, surface: surface, target: target, graphics: graphics, width: width, height: height,
            depth: depth, colorView: colorView, skyPipeline: skyPipeline, worldGroup: worldGroup,
            shadowMap: shadowMap, shadowGroup: shadowGroup, uniformBuffer: uniformBuffer,
            chunks: chunks, models: models, overlay: overlay, icons: icons,
            viewDistance: 160.0, fieldOfView: baseFieldOfView, drawnChunks: 0, chunkDraws: 0, gpuTimer: GpuTimer()
        ))
    }

    /// Match a new window size.
    mutating func resize(width: Int64, height: Int64) {
        self.width = width;
        self.height = height;
        guard width > 0 and height > 0 else {
            return;
        }
        self.surface.configure(device: self.device, format: self.target.format, width: width, height: height, presentMode: self.graphics.presentMode);
        self.depth = Renderer.makeDepth(self.device, self.target, width, height);
        self.colorView = Renderer.makeColor(self.device, self.target, width, height);
    }

    /// Draw `faces` for chunk (cx, cy, cz), lit by `lights`, replacing any
    /// it had.
    mutating func upload(cx: Int64, cy: Int64, cz: Int64, faces: ChunkFaces, lights: [Vec4]) {
        self.chunks.upload(self.device, cx, cy, cz, faces, lights);
    }

    /// The main pass's color attachment, cleared to `clear`: the
    /// multisampled buffer resolving into `frame`, or `frame` itself.
    func colorAttachment(frame: TextureView, clear clear: Color) -> ColorAttachment {
        match self.colorView {
            .Some(view) => view.colorAttachment(resolvingInto: frame, clear: .Some(clear)),
            .None => frame.colorAttachment(clear: .Some(clear))
        }
    }

    /// Draw one frame of `scene`.
    mutating func draw(scene: Scene) -> () throws WgpuError {
        // A hidden or collapsed window has nothing to draw into.
        guard self.width > 0 and self.height > 0 else {
            return .Ok(());
        }
        let light = scene.light;
        let (viewProj, lightViewProj) = self.writeUniforms(scene.view, eye: scene.eye, light: light, time: scene.time);
        let list = self.chunks.cull(Frustum(viewProj: viewProj), shadow: Frustum(viewProj: lightViewProj), eye: scene.eye);
        let quadCount = self.overlay.write(self.device, quads: scene.overlay, width: self.width, height: self.height);
        let iconCount = self.icons.write(self.device, boxes: scene.icons, width: self.width, height: self.height);
        let boxCount = self.models.write(self.device, boxes: scene.models);
        let shadowBoxes = scene.casters.clamp(0, boxCount);

        let frame = match self.surface.currentTexture() {
            .Ok(f) => f,
            .Err(.SurfaceOutdated(_)) => {
                self.surface.configure(device: self.device, format: self.target.format, width: self.width, height: self.height, presentMode: self.graphics.presentMode);
                return .Ok(());
            },
            .Err(.SurfaceTimeout(_)) => { return .Ok(()); },
            .Err(e) => { throw e }
        };
        let frameView = frame.texture.createView();
        let depthView = self.depth.createView();
        let shadowView = self.shadowMap.createView();
        let sky = Color(r: Float64(from: light.horizon.x), g: Float64(from: light.horizon.y), b: Float64(from: light.horizon.z));
        let encoder = self.device.createCommandEncoder();
        encoder.renderPass(RenderPassDescriptor(color: [], depth: .Some(shadowView.depthAttachment()), timestamps: self.gpuTimer.timestamps(.Shadow))) { (pass) in
            self.chunks.drawShadows(pass, list: list, uniforms: self.shadowGroup);
            self.models.drawShadows(pass, count: shadowBoxes, group: self.shadowGroup);
        };
        let target = self.colorAttachment(frameView, clear: sky);
        encoder.renderPass(RenderPassDescriptor(color: [target], depth: .Some(depthView.depthAttachment()), timestamps: self.gpuTimer.timestamps(.Main))) { (pass) in
            pass.setPipeline(self.skyPipeline);
            pass.setBindGroup(0, self.worldGroup);
            pass.draw(vertices: 3);
            self.chunks.drawOpaque(pass, list: list);
            self.models.draw(pass, count: boxCount);
            self.chunks.drawGlass(pass, list: list);
            self.overlay.draw(pass, count: quadCount);
            self.icons.draw(pass, count: iconCount, world: self.worldGroup);
        };
        self.drawnChunks = list.visible;
        self.chunkDraws = list.opaque.count;
        self.device.submit(encoder.finish());
        self.surface.present();
        try self.gpuTimer.collect(self.device);
        .Ok(())
    }

    /// Write the frame's uniforms; returns the camera's and the sun's
    /// view-projections.
    func writeUniforms(view: Mat4, eye eye: Vec3, light light: Daylight, time time: Float32) -> (Mat4, Mat4) {
        let aspect = Float32(from: self.width) / Float32(from: self.height);
        let projection = Mat4.perspective(fovY: radians(self.fieldOfView), aspect: aspect, near: 0.1, far: self.viewDistance + 64.0);
        let viewProj = projection * view;
        let lightViewProj = light.shadowView(eye, mapSize: self.graphics.quality.shadowSize);
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
        (viewProj, lightViewProj)
    }
}

/// Everything one frame shows.
struct Scene {
    /// The camera's view matrix, and where it is.
    var view: Mat4
    var eye: Vec3
    var light: Daylight
    /// Seconds since the start, which drifts the clouds.
    var time: Float32
    /// Boxes in the world; the first `casters` cast shadows.
    var models: [ModelBox]
    var casters: Int64
    /// The 2D interface, and the hotbar's 3D icons over it.
    var overlay: [OverlayQuad]
    var icons: [ModelBox]
}
