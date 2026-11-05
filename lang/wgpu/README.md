# wgpu

WebGPU for Kestrel, over [wgpu-native](https://github.com/gfx-rs/wgpu-native). There's no C shim: the bindings call `webgpu.h` directly, passing its structs by value and its callbacks as `CFunction`s.

Today it covers headless compute: adapters, devices, buffers, WGSL shaders, compute pipelines, bind groups, readback and error scopes. Render pipelines, textures and surfaces aren't bound yet. `wgpu.raw` shows the pattern for adding them.

## Installation

```toml
[dependencies]
wgpu = { path = "../../lang/wgpu" }
```

**Requires wgpu-native v27.** Download a release from [wgpu-native's releases](https://github.com/gfx-rs/wgpu-native/releases), unpack it, and point `WGPU_NATIVE_DIR` at it (the folder with `include/` and `lib/`). The build links `$WGPU_NATIVE_DIR/lib/libwgpu_native.a` statically, so the binary has no runtime dependency on it. `WGPU_NATIVE_DIR` defaults to `/usr/local`.

```sh
WGPU_NATIVE_DIR=~/wgpu-native flock build
```

Without a GPU, a software Vulkan driver works. On Debian or Ubuntu, `apt install mesa-vulkan-drivers` provides lavapipe.

## Usage

From `examples/wgpu_compute`, which doubles a list of numbers on the GPU:

```kestrel
import wgpu.(Instance, BufferUsage, WgpuError)

func run() -> () throws WgpuError {
    let instance = Instance();
    let adapter = try instance.requestAdapter();
    let device = try adapter.requestDevice();

    let data = device.createBuffer(size: 32, usage: [.Storage, .CopySrc, .CopyDst]);
    let readback = device.createBuffer(size: 32, usage: [.MapRead, .CopyDst]);
    device.write(data, data: input);

    let shaderModule = device.createShaderModule(wgsl: shader);
    let pipeline = device.createComputePipeline(shader: shaderModule, entryPoint: "main");
    let group = device.createBindGroup(pipeline: pipeline, group: 0, entries: [data.binding(0)]);

    let encoder = device.createCommandEncoder();
    encoder.computePass { (pass) in
        pass.setPipeline(pipeline);
        pass.setBindGroup(0, group);
        pass.dispatch(1);
    };
    encoder.copy(from: data, to: readback, size: 32);
    device.submit(encoder.finish());

    let result: [Float32] = try device.read(readback);
    .Ok(())
}
```

## Key types

Every handle is `not Copyable` and releases itself in `deinit`.

- **Instance**: `init()`, `requestAdapter(power:fallback:) throws WgpuError`. Pass `fallback: true` for a software adapter.
- **Adapter**: `info` (name, vendor, `AdapterType`), `requestDevice(label:) throws WgpuError`.
- **Device**: owns the queue.
  - Resources: `createBuffer(size:usage:label:)`, `createShaderModule(wgsl:label:)`, `createComputePipeline(shader:entryPoint:label:)`, `createBindGroup(pipeline:group:entries:label:)`, `createCommandEncoder()`.
  - Work: `write(buffer, data:offset:)`, `submit(commands)`, `wait()`.
  - Readback: `read[T](buffer) throws WgpuError` maps a `.MapRead` buffer and copies it out as `T`s.
  - Errors: `catchErrors(body) throws WgpuError` runs `body` in an error scope and returns its first error.
- **Buffer**: `size`, `binding(index)`.
- **CommandEncoder**: `computePass(body)`, `copy(from:to:size:)`, `finish()`.
- **ComputePass**: `setPipeline`, `setBindGroup`, `dispatch(x, y:, z:)`.
- **WgpuError**: `.NoAdapter`, `.NoDevice`, `.Validation`, `.OutOfMemory`, `.Internal` or `.MapFailed`, each with wgpu's message (`message`).

## Raw bindings

`wgpu.raw` mirrors `webgpu.h` names one to one (`WGPUBufferDescriptor`, `wgpuDeviceCreateBuffer`, `WGPUBufferUsage_Storage`, …). Every call needs `unsafe`.

- Descriptors are `FFISafe` structs passed as `&T`, which is C's `const T*`.
- `*CallbackInfo` structs pass by value.
- A callback is a `CFunction` of an `@export(.C)` function. Its state goes in `userdata1`.

See `src/wgpu.ks` for worked examples.
