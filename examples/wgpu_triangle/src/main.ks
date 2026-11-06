// Renders a colored triangle into an offscreen texture, reads the pixels
// back and writes them to `triangle.ppm`. Runs headless, so a software
// adapter such as lavapipe works.

module Main

import std.files.(Path, writeBytes)
import wgpu.(Instance, WgpuError, TextureFormat, TextureUsage, ColorTarget, VertexBufferLayout, VertexFormat, BufferUsage, Color)

let shader: String = "
struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
};

@vertex
fn vs_main(@location(0) position: vec2f, @location(1) color: vec3f) -> VertexOut {
    var out: VertexOut;
    out.position = vec4f(position, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    return vec4f(in.color, 1.0);
}
";

/// One vertex: a position and an RGB color, matching the shader's inputs.
struct Vertex: FFISafe {
    var x: Float32
    var y: Float32
    var r: Float32
    var g: Float32
    var b: Float32
}

let size: Int64 = 64;

func run() -> () throws WgpuError {
    let instance = Instance();
    let adapter = try instance.requestAdapter();
    let device = try adapter.requestDevice(label: "triangle");

    let target = device.createTexture(width: size, height: size, format: .RGBA8Unorm, usage: [.RenderAttachment, .CopySrc], label: "target");
    let targetView = target.createView();

    let vertices: [Vertex] = [
        Vertex(x: 0.0, y: 0.8, r: 1.0, g: 0.0, b: 0.0),
        Vertex(x: -0.8, y: -0.8, r: 0.0, g: 1.0, b: 0.0),
        Vertex(x: 0.8, y: -0.8, r: 0.0, g: 0.0, b: 1.0)
    ];
    let vertexBuffer = device.createBuffer(data: vertices, usage: [.Vertex], label: "vertices");

    let shaderModule = device.createShaderModule(wgsl: shader, label: "triangle");
    let pipeline = device.createRenderPipeline(
        shader: shaderModule,
        targets: [ColorTarget(format: .RGBA8Unorm)],
        buffers: [VertexBufferLayout(packed: [.Float32x2, .Float32x3])]
    );

    let encoder = device.createCommandEncoder();
    encoder.renderPass(color: [targetView.colorAttachment(clear: .Some(Color(r: 0.1, g: 0.1, b: 0.2)))]) { (pass) in
        pass.setPipeline(pipeline);
        pass.setVertexBuffer(0, vertexBuffer);
        pass.draw(vertices: 3);
    };
    device.submit(encoder.finish());

    let pixels = try device.read(texture: target);
    println("read \(pixels.count) bytes");
    printPixel(pixels, 32, 40, "center");
    printPixel(pixels, 0, 0, "corner");
    printPixel(pixels, 32, 8, "top");
    writePPM(pixels, "triangle.ppm");
    println("wrote triangle.ppm");
    .Ok(())
}

/// The RGB of pixel (`x`, `y`) of RGBA rows.
func printPixel(pixels: [UInt8], x: Int64, y: Int64, name: String) {
    let i = (y * size + x) * 4;
    println("\(name): \(pixels(i)) \(pixels(i + 1)) \(pixels(i + 2))");
}

/// Save RGBA rows as a binary PPM (which drops alpha).
func writePPM(pixels: [UInt8], path: String) {
    var bytes: [UInt8] = [];
    for b in "P6\n\(size) \(size)\n255\n".bytes {
        bytes.append(b);
    }
    var i = 0;
    while i < pixels.count {
        bytes.append(pixels(i));
        bytes.append(pixels(i + 1));
        bytes.append(pixels(i + 2));
        i = i + 4;
    }
    match writeBytes(Path(path), bytes.asSlice()) {
        .Ok(_) => {},
        .Err(_) => { println("could not write \(path)"); }
    }
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
