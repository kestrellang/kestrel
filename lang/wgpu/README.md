# wgpu

WebGPU for Kestrel, over [wgpu-native](https://github.com/gfx-rs/wgpu-native). There's no C shim: the bindings call `webgpu.h` directly, passing its structs by value and its callbacks as C function types (`@extern(.C) (…) -> …`).

It covers compute and rendering: adapters, devices, buffers, WGSL shaders, compute and render pipelines, textures, samplers, bind groups, depth buffers, texture and buffer readback, error scopes, and window surfaces (from any windowing library that gives native handles, such as `lang/sdl`'s `SDLWindow`). Vector and matrix math lives in `lang/linalg`.

Examples:

- `examples/wgpu_compute` doubles numbers with a compute shader.
- `examples/wgpu_triangle` renders a triangle offscreen and saves it as `triangle.ppm`.
- `examples/wgpu_cube` spins a depth-tested cube in an SDL window.
- `examples/wgpu_scene` flies a camera over a textured floor and a grid of instanced, lit cubes in an SDL window.

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
    let group = device.createBindGroup(layout: pipeline.bindGroupLayout(0), entries: [data.binding(0)]);

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

Rendering, from `examples/wgpu_triangle`:

```kestrel
let target = device.createTexture(width: 64, height: 64, format: .RGBA8Unorm, usage: [.RenderAttachment, .CopySrc]);
let targetView = target.createView();
let vertexBuffer = device.createBuffer(data: vertices, usage: [.Vertex]);
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
let pixels = try device.read(texture: target);   // RGBA bytes
```

Drawing to a window, from `examples/wgpu_cube`:

```kestrel
var window = SDLWindow(title: "Cube", width: 640, height: 480);
let surface = instance.createSurface(source);         // source from window.nativeWindow
let adapter = try instance.requestAdapter(compatibleWith: surface);
let format = surface.preferredFormat(adapter);
surface.configure(device: device, format: format, width: w, height: h);

// each frame
let frame = try surface.currentTexture();             // .SurfaceOutdated: configure again
let view = frame.texture.createView();
// ... a render pass into view.colorAttachment() ...
surface.present();
```

## Key types

Every handle is `not Copyable` and releases itself in `deinit`.

- **Instance**: `init()`, `requestAdapter(power:fallback:) throws WgpuError`. Pass `fallback: true` for a software adapter.
- **Adapter**: `info` (name, vendor, `AdapterType`), `requestDevice(label:) throws WgpuError`.
- **Instance** also: `requestAdapter(compatibleWith: surface, power:fallback:)`, `createSurface(source)`.
- **Device**: owns the queue.
  - Buffers: `createBuffer(size:usage:label:)`, `createBuffer(data:usage:label:)` (sized to and filled with `data`).
  - Shaders and pipelines: `createShaderModule(wgsl:label:)`, `createComputePipeline(shader:entryPoint:label:)`, `createRenderPipeline(shader:targets:buffers:vertexEntry:fragmentEntry:topology:cullMode:frontFace:depth:label:)`. Bind group layouts are inferred from the shader.
  - Textures: `createTexture(width:height:format:usage:label:)`, `createSampler(filter:addressMode:label:)`, `write(texture:data:)`.
  - Bind groups: `createBindGroup(layout:entries:label:)`, with the layout from `pipeline.bindGroupLayout(group)` and entries from `buffer.binding(n)`, `view.binding(n)`, `sampler.binding(n)`.
  - Work: `createCommandEncoder()`, `write(buffer, data:offset:)`, `write(buffer, value:offset:)` (one struct, such as a uniform), `submit(commands)`, `wait()`.
  - Readback: `read[T](buffer) throws WgpuError` maps a `.MapRead` buffer and copies it out as `T`s; `read(texture:) throws WgpuError` returns a color texture's bytes, rows tightly packed.
  - Errors: `catchErrors(body) throws WgpuError` runs `body` in an error scope and returns its first error.
- **Buffer**: `size`, `binding(index)`.
- **Texture**: `width`, `height`, `format`, `paddedBytesPerRow`, `createView()`.
- **TextureView**: `binding(index)`, `colorAttachment(clear:)` (`None` keeps the contents), `depthAttachment(clear:)`.
- **RenderPipeline**, **ComputePipeline**: `bindGroupLayout(group)`.
- **VertexBufferLayout**: `init(stride:attributes:stepMode:)` or `init(packed: [VertexFormat])` for fields back to back at locations 0, 1, ….
- **ColorTarget** (`format`, `blend: .Replace` or `.AlphaBlending`), **DepthTest** (`format`, `compare`, `write`).
- **CommandEncoder**: `computePass(body)`, `renderPass(color:body)`, `renderPass(color:depth:body)`, `copy(from:to:size:)`, `copy(texture:to:)`, `finish()`.
- **ComputePass**: `setPipeline`, `setBindGroup`, `dispatch(x, y:, z:)`.
- **RenderPass**: `setPipeline`, `setBindGroup`, `setVertexBuffer(slot, buffer)`, `setIndexBuffer(buffer, format:)`, `setViewport`, `setScissor`, `draw(vertices:instances:firstVertex:)`, `drawIndexed(indices:instances:firstIndex:)`.
- **Surface**: `preferredFormat(adapter)`, `configure(device:format:width:height:presentMode:)`, `currentTexture() throws WgpuError` (a `SurfaceTexture` with `texture` and `suboptimal`), `present()`.
- **SurfaceSource**: `.Xlib(display, window)`, `.Wayland(display, surface)`, `.MetalLayer(layer)`, `.Windows(hinstance, hwnd)`.
- **WgpuError**: `.NoAdapter`, `.NoDevice`, `.Validation`, `.OutOfMemory`, `.Internal`, `.MapFailed`, `.SurfaceOutdated`, `.SurfaceLost` or `.SurfaceTimeout`, each with a message (`message`).

## Raw bindings

`wgpu.raw` mirrors `webgpu.h` names one to one (`WGPUBufferDescriptor`, `wgpuDeviceCreateBuffer`, `WGPUBufferUsage_Storage`, …). Every call needs `unsafe`.

- Descriptors are `FFISafe` structs passed as `&T`, which is C's `const T*`.
- `*CallbackInfo` structs pass by value.
- A callback is an `@export(.C)` function, passed as a `NullableCFunction` in the `*CallbackInfo` field. Its state goes in `userdata1`.

See `src/wgpu.ks` for worked examples.
