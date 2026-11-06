// A spinning cube in a window: an SDL window, a wgpu surface on it, a depth
// buffer, and a model-view-projection matrix from `linalg` in a uniform
// buffer. Close the window or press Escape to quit. `CUBE_FRAMES=n` quits
// after n frames.

module Main

import wgpu.(Instance, Adapter, Device, Surface, SurfaceSource, Texture, TextureFormat, TextureUsage, WgpuError, ColorTarget, VertexBufferLayout, VertexFormat, BufferUsage, Color, DepthTest, CullMode, IndexFormat)
import sdl.(SDLWindow, NativeWindow, Event, Key, Milliseconds, monotonicMs)
import linalg.(Vec3, Mat4, radians)

let shader: String = "
struct Uniforms {
    mvp: mat4x4f,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
};

@vertex
fn vs_main(@location(0) position: vec3f, @location(1) color: vec3f) -> VertexOut {
    var out: VertexOut;
    out.position = uniforms.mvp * vec4f(position, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    return vec4f(in.color, 1.0);
}
";

/// A corner of the cube, colored by where it sits.
struct Vertex: FFISafe {
    var position: Vec3
    var color: Vec3
}

/// The cube's 8 corners.
func cubeVertices() -> [Vertex] {
    var out: [Vertex] = [];
    var i = 0;
    while i < 8 {
        let p = Vec3(
            x: if i % 2 == 0 { -1.0 } else { 1.0 },
            y: if (i / 2) % 2 == 0 { -1.0 } else { 1.0 },
            z: if i / 4 == 0 { -1.0 } else { 1.0 }
        );
        out.append(Vertex(position: p, color: p * 0.5 + Vec3(splat: 0.5)));
        i = i + 1;
    }
    out
}

/// Two counter-clockwise triangles per face, seen from outside.
let cubeIndices: [UInt16] = [
    0, 2, 3, 0, 3, 1,  // -z
    4, 5, 7, 4, 7, 6,  // +z
    0, 4, 6, 0, 6, 2,  // -x
    1, 3, 7, 1, 7, 5,  // +x
    0, 1, 5, 0, 5, 4,  // -y
    2, 6, 7, 2, 7, 3   // +y
];

let depthFormat: TextureFormat = .Depth24Plus;

/// The surface source for `window`'s platform handles.
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

func run() -> () throws WgpuError {
    var window = SDLWindow(title: "Kestrel wgpu cube", width: 640, height: 480);
    let instance = Instance();
    let surface = instance.createSurface(surfaceSource(window.nativeWindow));
    let adapter = try instance.requestAdapter(compatibleWith: surface);
    let info = adapter.info;
    println("adapter: \(info.name)");
    let device = try adapter.requestDevice(label: "cube");

    let format = surface.preferredFormat(adapter);
    var (width, height) = window.drawableSize;
    surface.configure(device: device, format: format, width: width, height: height);
    var depth = makeDepth(device, width, height);

    let vertexBuffer = device.createBuffer(data: cubeVertices(), usage: [.Vertex], label: "vertices");
    let indexBuffer = device.createBuffer(data: cubeIndices, usage: [.Index], label: "indices");
    let uniformBuffer = device.createBuffer(size: 64, usage: [.Uniform, .CopyDst], label: "uniforms");

    let shaderModule = device.createShaderModule(wgsl: shader, label: "cube");
    let pipeline = device.createRenderPipeline(
        shader: shaderModule,
        targets: [ColorTarget(format: format)],
        buffers: [VertexBufferLayout(packed: [.Float32x3, .Float32x3])],
        cullMode: .Back,
        depth: .Some(DepthTest(format: depthFormat))
    );
    let bindGroup = device.createBindGroup(layout: pipeline.bindGroupLayout(0), entries: [uniformBuffer.binding(0)]);

    let maxFrames = match getenv("CUBE_FRAMES") {
        .Some(s) => Int64(parsing: s) ?? 0,
        .None => 0
    };
    let start = unsafe monotonicMs();
    var frames = 0;
    var running = true;
    while running {
        while let .Some(event) = window.pollEvent() {
            match event {
                .Quit => { running = false },
                .KeyDown(.Escape) => { running = false },
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

        let seconds = Float32(from: unsafe monotonicMs() - start) / 1000.0;
        let model = Mat4.rotation(axis: Vec3(x: 1.0, y: 1.0, z: 0.0), angle: seconds);
        let view = Mat4.lookAt(eye: Vec3(x: 0.0, y: 0.0, z: 5.0), target: Vec3.zero, up: Vec3.unitY);
        let aspect = Float32(from: width) / Float32(from: height);
        let projection = Mat4.perspective(fovY: radians(60.0), aspect: aspect, near: 0.1, far: 100.0);
        device.write(uniformBuffer, value: projection * view * model);

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
        encoder.renderPass(
            color: [frameView.colorAttachment(clear: .Some(Color(r: 0.05, g: 0.05, b: 0.1)))],
            depth: depthView.depthAttachment()
        ) { (pass) in
            pass.setPipeline(pipeline);
            pass.setBindGroup(0, bindGroup);
            pass.setVertexBuffer(0, vertexBuffer);
            pass.setIndexBuffer(indexBuffer, format: .Uint16);
            pass.drawIndexed(indices: 36);
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
