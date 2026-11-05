// A lit 3D scene: a checkerboard floor and a grid of spinning cubes drawn
// with one instanced draw, lit by a slowly circling sun, with specular
// highlights and distance fog. Fly with WASD, Space and Left Shift; look
// with the arrow keys; Escape quits. `SCENE_FRAMES=n` quits after n frames.
//
// It shows the parts of `wgpu` the cube example doesn't: a texture uploaded
// from memory and read through a sampler, a second vertex buffer stepped per
// instance, several draws from one buffer, and a bind group with a uniform
// buffer, a texture and a sampler.

module Main

import wgpu.(Instance, Device, SurfaceSource, Texture, TextureFormat, WgpuError, ColorTarget, VertexBufferLayout, VertexAttribute, VertexFormat, Color, DepthTest)
import sdl.(SDLWindow, NativeWindow, Event, Key, monotonicMs)
import linalg.(Vec2, Vec3, Vec4, Mat4, radians)

let shader: String = "
struct Uniforms {
    viewProj: mat4x4f,
    lightDir: vec4f,
    camera: vec4f,
    fog: vec4f,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) world: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
    @location(3) color: vec4f,
};

@vertex
fn vs_main(
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
    @location(3) m0: vec4f,
    @location(4) m1: vec4f,
    @location(5) m2: vec4f,
    @location(6) m3: vec4f,
    @location(7) color: vec4f,
) -> VertexOut {
    let model = mat4x4f(m0, m1, m2, m3);
    let world = model * vec4f(position, 1.0);
    var out: VertexOut;
    out.position = u.viewProj * world;
    out.world = world.xyz;
    out.normal = (model * vec4f(normal, 0.0)).xyz;
    out.uv = uv;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    let n = normalize(in.normal);
    let l = normalize(-u.lightDir.xyz);
    let v = normalize(u.camera.xyz - in.world);
    let h = normalize(l + v);
    // The instance color's alpha says how much of the texture shows.
    let texel = textureSample(tex, samp, in.uv).rgb;
    let albedo = in.color.rgb * mix(vec3f(1.0), texel, in.color.a);
    let diffuse = max(dot(n, l), 0.0);
    let specular = pow(max(dot(n, h), 0.0), 48.0) * 0.35 * step(0.0, diffuse);
    // Warm light from below, cool from the sky above.
    let ambient = mix(vec3f(0.18, 0.15, 0.13), vec3f(0.30, 0.36, 0.48), n.y * 0.5 + 0.5);
    var color = albedo * (ambient + diffuse * vec3f(1.0, 0.94, 0.82)) + vec3f(specular);
    let distance = length(u.camera.xyz - in.world);
    color = mix(color, u.fog.rgb, 1.0 - exp(-distance * u.fog.w));
    return vec4f(color, 1.0);
}
";

/// A mesh vertex: where it is, which way it faces, and where it samples.
struct Vertex: FFISafe {
    var position: Vec3
    var normal: Vec3
    var uv: Vec2
}

/// One drawn copy of the mesh: its model matrix (column by column, the way
/// the shader reads it) and its color, with the texture's weight in `w`.
struct InstanceData: FFISafe {
    var model: Mat4
    var color: Vec4
}

/// What the shader's `Uniforms` holds, laid out the same way.
struct Uniforms: FFISafe {
    var viewProj: Mat4
    var lightDir: Vec4
    var camera: Vec4
    var fog: Vec4
}

/// The shared mesh: a unit cube's 24 vertices (four per face, so each face
/// has its own normal) then the floor's 4.
struct Mesh {
    var vertices: [Vertex]
    var indices: [UInt16]
}

let cubeIndexCount: Int64 = 36;
let floorIndexCount: Int64 = 6;
let floorSize: Float32 = 120.0;

/// Add a square facing `normal`, spanned by `u` and `v` (with `u × v` along
/// `normal`, so it winds counter-clockwise from outside), with texture
/// coordinates scaled by `tiles`.
func addFace(mesh: Mesh, center center: Vec3, normal normal: Vec3, u u: Vec3, v v: Vec3, tiles tiles: Float32) -> Mesh {
    var out = mesh;
    let base = UInt16(from: out.vertices.count);
    let corners: [(Float32, Float32)] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    for (a, b) in corners {
        out.vertices.append(Vertex(
            position: center + u * a + v * b,
            normal: normal,
            uv: Vec2(x: (a + 1.0) * 0.5 * tiles, y: (1.0 - b) * 0.5 * tiles)
        ));
    }
    for i in [0, 1, 2, 0, 2, 3] {
        out.indices.append(base + UInt16(from: i));
    }
    out
}

func buildMesh() -> Mesh {
    var mesh = Mesh(vertices: [], indices: []);
    // (normal, u, v) for each cube face.
    let faces: [(Vec3, Vec3, Vec3)] = [
        (Vec3.unitX, -Vec3.unitZ, Vec3.unitY),
        (-Vec3.unitX, Vec3.unitZ, Vec3.unitY),
        (Vec3.unitY, Vec3.unitX, -Vec3.unitZ),
        (-Vec3.unitY, Vec3.unitX, Vec3.unitZ),
        (Vec3.unitZ, Vec3.unitX, Vec3.unitY),
        (-Vec3.unitZ, -Vec3.unitX, Vec3.unitY)
    ];
    for (n, u, v) in faces {
        mesh = addFace(mesh, center: n, normal: n, u: u, v: v, tiles: 1.0);
    }
    let half = floorSize * 0.5;
    addFace(mesh, center: Vec3.zero, normal: Vec3.unitY, u: Vec3.unitX * half, v: -Vec3.unitZ * half, tiles: floorSize / 2.0)
}

/// A 64x64 checkerboard of four 32-pixel tiles with darker grout lines,
/// as RGBA bytes.
func checkerboard() -> [UInt8] {
    var pixels: [UInt8] = [];
    var y = 0;
    while y < 64 {
        var x = 0;
        while x < 64 {
            let light = ((x / 32) + (y / 32)) % 2 == 0;
            let grout = x % 32 == 0 or y % 32 == 0;
            let shade: UInt8 = if grout { 90 } else if light { 235 } else { 170 };
            pixels.append(shade);
            pixels.append(shade);
            pixels.append(shade);
            pixels.append(255);
            x = x + 1;
        }
        y = y + 1;
    }
    pixels
}

let gridSide: Int64 = 7;
let spacing: Float32 = 3.0;

/// A smooth rainbow: hue 0 to 1 around the color wheel, as linear color
/// (squared, so it stays saturated once the sRGB surface brightens it).
func hue(h: Float32) -> Vec3 {
    let tau = Float32.pi * 2.0;
    let r = 0.5 + 0.5 * (tau * h).cos();
    let g = 0.5 + 0.5 * (tau * (h - 0.33)).cos();
    let b = 0.5 + 0.5 * (tau * (h - 0.67)).cos();
    Vec3(x: r * r, y: g * g, z: b * b)
}

/// The cubes at `seconds`: a grid, each bobbing and spinning at its own pace.
func cubeInstances(seconds: Float32) -> [InstanceData] {
    var out: [InstanceData] = [];
    let offset = Float32(from: gridSide - 1) * 0.5;
    var row = 0;
    while row < gridSide {
        var column = 0;
        while column < gridSide {
            let i = Float32(from: row * gridSide + column);
            let x = (Float32(from: column) - offset) * spacing;
            let z = (Float32(from: row) - offset) * spacing;
            let size = 0.45 + 0.25 * (i * 1.7).sin().abs();
            let lift = size + 0.6 + 0.5 * (seconds * 1.3 + i * 0.6).sin();
            let spin = seconds * (0.4 + 0.08 * Float32(from: (row + column) % 5));
            let model = Mat4.translation(Vec3(x: x, y: lift, z: z))
                * Mat4.rotation(axis: Vec3(x: 0.3, y: 1.0, z: 0.2), angle: spin + i)
                * Mat4.scaling(Vec3(splat: size));
            let color = hue(i / Float32(from: gridSide * gridSide)).extended(w: 0.25);
            out.append(InstanceData(model: model, color: color));
            column = column + 1;
        }
        row = row + 1;
    }
    out
}

/// Which movement keys are held.
struct Controls {
    var forward: Bool = false
    var back: Bool = false
    var left: Bool = false
    var right: Bool = false
    var up: Bool = false
    var down: Bool = false
    var turnLeft: Bool = false
    var turnRight: Bool = false
    var lookUp: Bool = false
    var lookDown: Bool = false

    init() {}

    mutating func setKey(key: Key, held held: Bool) {
        match key {
            .W => { self.forward = held },
            .S => { self.back = held },
            .A => { self.left = held },
            .D => { self.right = held },
            .Space => { self.up = held },
            .LeftShift => { self.down = held },
            .LeftArrow => { self.turnLeft = held },
            .RightArrow => { self.turnRight = held },
            .UpArrow => { self.lookUp = held },
            .DownArrow => { self.lookDown = held },
            _ => {}
        }
    }
}

func axis(positive: Bool, negative: Bool) -> Float32 {
    (if positive { 1.0 } else { 0.0 }) - (if negative { 1.0 } else { 0.0 })
}

/// A camera at `position`, turned `yaw` radians from looking down -z and
/// tilted `pitch` radians up.
struct Camera {
    var position: Vec3
    var yaw: Float32
    var pitch: Float32

    var forward: Vec3 {
        Vec3(
            x: self.yaw.sin() * self.pitch.cos(),
            y: self.pitch.sin(),
            z: -self.yaw.cos() * self.pitch.cos()
        )
    }

    /// Move and turn for `dt` seconds of the held `controls`.
    mutating func update(controls: Controls, dt dt: Float32) {
        self.yaw = self.yaw + axis(controls.turnRight, controls.turnLeft) * 1.6 * dt;
        let limit = 1.5;
        self.pitch = (self.pitch + axis(controls.lookUp, controls.lookDown) * 1.2 * dt).clamp(-limit, limit);
        // Walk on the ground plane, whatever the pitch.
        let ahead = Vec3(x: self.yaw.sin(), y: 0.0, z: -self.yaw.cos());
        let side = ahead.cross(Vec3.unitY);
        let move = ahead * axis(controls.forward, controls.back)
            + side * axis(controls.right, controls.left)
            + Vec3.unitY * axis(controls.up, controls.down);
        self.position = self.position + move * (7.0 * dt);
    }

    func view() -> Mat4 {
        Mat4.lookAt(eye: self.position, target: self.position + self.forward, up: Vec3.unitY)
    }
}

let depthFormat: TextureFormat = .Depth24Plus;
let fogColor: Vec3 = Vec3(x: 0.42, y: 0.52, z: 0.68);

func surfaceSource(window: NativeWindow) -> SurfaceSource {
    match window {
        .Xlib(display, id) => .Xlib(display, id),
        .Wayland(display, surface) => .Wayland(display, surface),
        .MetalLayer(layer) => .MetalLayer(layer),
        .Windows(hinstance, hwnd) => .Windows(hinstance, hwnd),
        .Unsupported => fatalError("this platform's windows aren't supported")
    }
}

func makeDepth(device: Device, width: Int64, height: Int64) -> Texture {
    device.createTexture(width: width, height: height, format: depthFormat, usage: [.RenderAttachment], label: "depth")
}

/// The per-instance buffer layout: four columns of the model matrix, then
/// the color, at shader locations 3 through 7.
func instanceLayout() -> VertexBufferLayout {
    var attributes: [VertexAttribute] = [];
    var i = 0;
    while i < 5 {
        attributes.append(VertexAttribute(format: .Float32x4, offset: i * 16, location: 3 + i));
        i = i + 1;
    }
    VertexBufferLayout(stride: 80, attributes: attributes, stepMode: .Instance)
}

func run() -> () throws WgpuError {
    var window = SDLWindow(title: "Kestrel wgpu scene", width: 960, height: 600);
    let instance = Instance();
    let surface = instance.createSurface(surfaceSource(window.nativeWindow));
    let adapter = try instance.requestAdapter(compatibleWith: surface);
    let info = adapter.info;
    println("adapter: \(info.name)");
    let device = try adapter.requestDevice(label: "scene");

    let format = surface.preferredFormat(adapter);
    var (width, height) = window.drawableSize;
    surface.configure(device: device, format: format, width: width, height: height);
    var depth = makeDepth(device, width, height);

    // Geometry: one vertex and index buffer for both the cubes and the floor.
    let mesh = buildMesh();
    let vertexBuffer = device.createBuffer(data: mesh.vertices, usage: [.Vertex], label: "mesh vertices");
    let indexBuffer = device.createBuffer(data: mesh.indices, usage: [.Index], label: "mesh indices");
    let floorInstance = device.createBuffer(
        data: [InstanceData(model: Mat4.identity, color: Vec4(x: 0.9, y: 0.88, z: 0.84, w: 1.0))],
        usage: [.Vertex],
        label: "floor instance"
    );
    let cubeCount = gridSide * gridSide;
    let cubeBuffer = device.createBuffer(size: cubeCount * 80, usage: [.Vertex, .CopyDst], label: "cube instances");
    let uniformBuffer = device.createBuffer(size: 112, usage: [.Uniform, .CopyDst], label: "uniforms");

    // The floor texture, uploaded once.
    let texture = device.createTexture(width: 64, height: 64, format: .RGBA8UnormSrgb, usage: [.TextureBinding, .CopyDst], label: "checkerboard");
    device.write(texture: texture, data: checkerboard());
    let textureView = texture.createView();
    let sampler = device.createSampler(filter: .Linear, addressMode: .Repeat, label: "repeat");

    let shaderModule = device.createShaderModule(wgsl: shader, label: "scene");
    let pipeline = device.createRenderPipeline(
        shader: shaderModule,
        targets: [ColorTarget(format: format)],
        buffers: [VertexBufferLayout(packed: [.Float32x3, .Float32x3, .Float32x2]), instanceLayout()],
        cullMode: .Back,
        depth: .Some(DepthTest(format: depthFormat))
    );
    let bindGroup = device.createBindGroup(
        layout: pipeline.bindGroupLayout(0),
        entries: [uniformBuffer.binding(0), textureView.binding(1), sampler.binding(2)]
    );

    var camera = Camera(position: Vec3(x: 0.0, y: 5.0, z: 17.0), yaw: 0.0, pitch: -0.28);
    var controls = Controls();
    let maxFrames = match getenv("SCENE_FRAMES") {
        .Some(s) => Int64(parsing: s) ?? 0,
        .None => 0
    };
    let start = unsafe monotonicMs();
    var last = start;
    var frames = 0;
    var running = true;
    while running {
        while let .Some(event) = window.pollEvent() {
            match event {
                .Quit => { running = false },
                .KeyDown(.Escape) => { running = false },
                .KeyDown(key) => { controls.setKey(key, held: true) },
                .KeyUp(key) => { controls.setKey(key, held: false) },
                .Resized(_, _) => {
                    let (w, h) = window.drawableSize;
                    width = w;
                    height = h;
                    surface.configure(device: device, format: format, width: width, height: height);
                    depth = makeDepth(device, width, height);
                },
                _ => {}
            }
        }

        let now = unsafe monotonicMs();
        let dt = Float32(from: now - last) / 1000.0;
        last = now;
        let seconds = Float32(from: now - start) / 1000.0;
        camera.update(controls, dt: dt);

        let aspect = Float32(from: width) / Float32(from: height);
        let projection = Mat4.perspective(fovY: radians(60.0), aspect: aspect, near: 0.1, far: 200.0);
        let sunAngle = seconds * 0.25;
        let sun = Vec3(x: sunAngle.cos(), y: -1.3, z: sunAngle.sin()).normalized;
        device.write(uniformBuffer, value: Uniforms(
            viewProj: projection * camera.view(),
            lightDir: sun.extended(w: 0.0),
            camera: camera.position.extended(w: 1.0),
            fog: fogColor.extended(w: 0.018)
        ));
        device.write(cubeBuffer, data: cubeInstances(seconds));

        let frame = match surface.currentTexture() {
            .Ok(f) => f,
            .Err(.SurfaceOutdated(_)) => {
                surface.configure(device: device, format: format, width: width, height: height);
                continue
            },
            .Err(.SurfaceTimeout(_)) => { continue },
            .Err(e) => { throw e }
        };
        let frameView = frame.texture.createView();
        let depthView = depth.createView();
        let encoder = device.createCommandEncoder();
        let sky = Color(r: Float64(from: fogColor.x), g: Float64(from: fogColor.y), b: Float64(from: fogColor.z));
        encoder.renderPass(color: [frameView.colorAttachment(clear: .Some(sky))], depth: depthView.depthAttachment()) { (pass) in
            pass.setPipeline(pipeline);
            pass.setBindGroup(0, bindGroup);
            pass.setVertexBuffer(0, vertexBuffer);
            pass.setIndexBuffer(indexBuffer, format: .Uint16);
            // The floor's indices come after the cube's in the shared buffer.
            pass.setVertexBuffer(1, floorInstance);
            pass.drawIndexed(indices: floorIndexCount, firstIndex: cubeIndexCount);
            pass.setVertexBuffer(1, cubeBuffer);
            pass.drawIndexed(indices: cubeIndexCount, instances: cubeCount);
        };
        device.submit(encoder.finish());
        surface.present();

        frames = frames + 1;
        if maxFrames > 0 and frames >= maxFrames {
            running = false;
        }
    }
    println("drew \(frames) frames");
    .Ok(())
}

@main
func main() -> lang.i64 {
    match run() {
        .Ok(_) => 0,
        .Err(e) => {
            println("wgpu error: \(e.message)");
            1
        }
    }
}
