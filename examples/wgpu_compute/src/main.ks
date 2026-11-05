// Doubles a list of numbers on the GPU with a WGSL compute shader. Runs
// headless, so a software adapter such as lavapipe works.

module Main

import wgpu.(Instance, BufferUsage, WgpuError)

let shader: String = "
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&data) {
        data[id.x] = data[id.x] * 2.0 + 1.0;
    }
}
";

func run() -> () throws WgpuError {
    let instance = Instance();
    let adapter = try instance.requestAdapter();
    let info = adapter.info;
    println("adapter: \(info.name)");
    let device = try adapter.requestDevice(label: "compute");

    var input: [Float32] = [];
    var i = 0;
    while i < 8 {
        input.append(Float32(from: i));
        i = i + 1;
    }
    let size: Int64 = 8 * 4;
    let data = device.createBuffer(size: size, usage: [.Storage, .CopySrc, .CopyDst], label: "data");
    let readback = device.createBuffer(size: size, usage: [.MapRead, .CopyDst], label: "readback");
    device.write(data, data: input);

    let shaderModule = device.createShaderModule(wgsl: shader, label: "double");
    let pipeline = device.createComputePipeline(shader: shaderModule, entryPoint: "main");
    let group = device.createBindGroup(pipeline: pipeline, group: 0, entries: [data.binding(0)]);

    let encoder = device.createCommandEncoder();
    encoder.computePass { (pass) in
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, group);
        pass.dispatch(1);
    };
    encoder.copy(from: data, to: readback, size: size);
    device.submit(encoder.finish());

    let result: [Float32] = try device.read(readback);
    for x in result {
        println(x);
    }

    // A broken shader is a validation error, caught instead of aborting.
    let checked = device.catchErrors { device.createShaderModule(wgsl: "not wgsl"); };
    match checked {
        .Ok(_) => println("no error?"),
        .Err(_) => println("caught: shader rejected")
    }
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
