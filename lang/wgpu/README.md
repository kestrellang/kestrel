# wgpu

WebGPU for Kestrel, over [wgpu-native](https://github.com/gfx-rs/wgpu-native). There's no C shim: the bindings call `webgpu.h` directly, passing its structs by value and its callbacks as C function types (`@extern(.C) (…) -> …`).

It covers compute and rendering: adapters, devices, features and limits, buffers, WGSL shaders, compute and render pipelines, explicit bind group and pipeline layouts, push constants, textures (2D, 3D, array layers, cube maps, mip chains, multisampling), mipmap generation, samplers (comparison and anisotropic), blending, write masks, depth and stencil, indexed, instanced and indirect draws, dynamic offsets, render bundles, occlusion and timestamp queries, texture and buffer readback, error scopes, and window surfaces (from any windowing library that gives native handles, such as `lang/sdl`'s `SDLWindow`). Vector and matrix math lives in `lang/linalg`.

Examples:

- `examples/wgpu_compute` doubles numbers with a compute shader.
- `examples/wgpu_triangle` renders a triangle offscreen and saves it as `triangle.ppm`.
- `examples/wgpu_cube` spins a depth-tested cube in an SDL window.
- `examples/wgpu_scene` flies a camera over a mipmapped floor and a grid of instanced, lit cubes in an SDL window, with shadow mapping and 4x MSAA.
- `examples/wgpu_features` runs 14 headless checks, one per feature (MSAA, mipmaps, layers, stencil, blending, layouts, indirect draws, bundles, queries, push constants, cube maps, …), and reads each result back.

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

### In a browser

The package also builds for `wasm64-unknown-emscripten`, against the browser's WebGPU through Dawn's `emdawnwebgpu` port: link with `--link-arg=--use-port=emdawnwebgpu --link-arg=-sDEFAULT_TO_CXX --link-arg=-sASYNCIFY`. Make the surface with `SurfaceSource.Canvas("#canvas")`. Waiting for the GPU yields to the browser instead of blocking, `present()` does nothing (the browser shows the frame when control returns to it), and push constants, a wgpu-native extension, are unavailable. `examples/aerie/web/build.sh` is a complete example.

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

Every handle is `not Copyable` and releases itself in `deinit`. Flags are arrays: `usage: [.Storage, .CopyDst]`, `visibility: [.Vertex, .Fragment]`.

- **Instance**: `init()`, `requestAdapter(power:fallback:) throws WgpuError` (`fallback: true` for a software adapter), `requestAdapter(compatibleWith: surface, power:fallback:)`, `createSurface(source)`.
- **Adapter**: `info` (name, vendor, `AdapterType`), `supports(feature)`, `limits`, `requestDevice(label:features:maxPushConstantSize:) throws WgpuError`. A nonzero `maxPushConstantSize` turns on push constants.
- **Device**: owns the queue. Also `supports(feature)` and `limits`.
  - Buffers: `createBuffer(size:usage:label:)`, `createBuffer(data:usage:label:)` (sized to and filled with `data`).
  - Shaders and pipelines: `createShaderModule(wgsl:label:)`, `createComputePipeline(shader:entryPoint:layout:label:)`, `createRenderPipeline(shader:targets:buffers:vertexEntry:fragmentEntry:topology:cullMode:frontFace:depth:sampleCount:alphaToCoverage:layout:label:)`. Without `layout`, bind group layouts are inferred from the shader. With no `targets`, the pipeline is depth-only (for shadow maps).
  - Layouts: `createBindGroupLayout(entries:label:)` and `createPipelineLayout(groups:pushConstants:label:)`, from `BindingLayout.uniformBuffer`, `.storageBuffer`, `.texture`, `.storageTexture` and `.sampler` (each with `visibility`, and `dynamic:` for buffers).
  - Textures: `createTexture(width:height:format:usage:layers:dimension:mipLevels:sampleCount:label:)`, `write(texture:data:mipLevel:layer:)`, `generateMipmaps(texture)` (render-attachment color textures).
  - Samplers: `createSampler(filter:addressMode:mipmapFilter:compare:anisotropy:lodMinClamp:lodMaxClamp:label:)`. Set `compare` for a `sampler_comparison`.
  - Bind groups: `createBindGroup(layout:entries:label:)`, with the layout from `pipeline.bindGroupLayout(group)` or `pipelineLayout.bindGroupLayout(group)`, and entries from `buffer.binding(n, offset:size:)`, `view.binding(n)`, `sampler.binding(n)`.
  - Render bundles: `createRenderBundle(targets, body)` records draws once with a `RenderBundleEncoder`, for `pass.execute(bundle)`.
  - Queries: `createQuerySet(kind:count:label:)`, `readQueries(set) throws WgpuError` (sample counts or nanoseconds).
  - Work: `createCommandEncoder()`, `write(buffer, data:offset:)`, `write(buffer, value:offset:)` (one struct, such as a uniform), `submit(commands)`, `wait()`.
  - Readback: `read[T](buffer) throws WgpuError` maps a `.MapRead` buffer and copies it out as `T`s; `read(texture:mipLevel:) throws WgpuError` returns a color texture's bytes, rows tightly packed.
  - Errors: `catchErrors(body) throws WgpuError` runs `body` in an error scope and returns its first error.
- **Buffer**: `size`, `binding(index, offset:size:)`.
- **Texture**: `width`, `height`, `layers`, `mipLevelCount`, `sampleCount`, `dimension`, `format`, `levelWidth(level)`, `levelHeight(level)`, `paddedRowBytes(level)`, `Texture.fullMipChain(width:height:)`, `createView()`, `createView(range:label:)` with a `ViewRange` (`.level(n, layer:)`, `.cube()`, or any dimension, mip and layer range).
- **TextureView**: `binding(index)`, `colorAttachment(clear:)` (`None` keeps the contents), `colorAttachment(resolvingInto: view, clear:)` for MSAA, `depthAttachment(clear:stencilClear:readOnly:)`.
- **RenderPipeline**, **ComputePipeline**, **PipelineLayout**: `bindGroupLayout(group)`. `PipelineLayout.share()` and `BindGroupLayout.share()` give a second handle to the same object.
- **VertexBufferLayout**: `init(stride:attributes:stepMode:)` or `init(packed: [VertexFormat])` for fields back to back at locations 0, 1, ….
- **ColorTarget**: `format`, `blend` (`.Replace`, `.AlphaBlending`, `.PremultipliedAlpha`, `.Additive`, or `.Custom(color, alpha)` from `BlendComponent(source:destination:operation:)`), `writeMask: [ColorChannel]`.
- **DepthTest**: `format`, `compare`, `write`, `stencil: StencilTest?` (`StencilTest(both: StencilFace(compare:fail:depthFail:pass:))` or separate `front:back:`), `depthBias`, `depthBiasSlopeScale`, `depthBiasClamp`.
- **CommandEncoder**: `computePass(body)`, `computePass(timestamps:body)`, `renderPass(color:body)`, `renderPass(color:depth:body)`, `renderPass(RenderPassDescriptor(color:depth:occlusionQueries:timestamps:label:), body)`, `copy(from:to:size:)`, `copy(texture:to:mipLevel:layer:)`, `copy(buffer:toTexture:bytesPerRow:mipLevel:layer:)`, `copy(texture:toTexture:mipLevel:fromLayer:toLayer:)`, `resolve(querySet, into:first:count:offset:)`, `finish()`.
- **ComputePass**: `setPipeline`, `setBindGroup(index, group, dynamicOffsets:)`, `setPushConstants(value:offset:)`, `dispatch(x, y:, z:)`, `dispatchIndirect(buffer, offset:)`.
- **RenderPass** (and **RenderBundleEncoder**, minus the pass-only calls): `setPipeline`, `setBindGroup(index, group, dynamicOffsets:)`, `setVertexBuffer(slot, buffer, offset:size:)`, `setIndexBuffer(buffer, format:offset:size:)`, `setPushConstants(stages:value:offset:)`, `draw(vertices:instances:firstVertex:firstInstance:)`, `drawIndexed(indices:instances:firstIndex:baseVertex:firstInstance:)`, `drawIndirect(buffer, offset:)` and `drawIndexedIndirect` (fill the buffer with `DrawIndirectArgs` or `DrawIndexedIndirectArgs`). Pass-only: `setViewport`, `setScissor`, `setStencilReference`, `setBlendConstant`, `beginOcclusionQuery(index)`, `endOcclusionQuery()`, `execute(bundle)`.
- **QuerySet**: `count`, `kind`, `occlusionQueries` and `timestamps(beginning:end:)` for pass descriptors. Timestamps need `Feature.TimestampQuery`.
- **Feature**: `.DepthClipControl`, `.Depth32FloatStencil8`, `.TimestampQuery`, `.IndirectFirstInstance`, `.ShaderF16`, `.Float32Filterable`, `.PushConstants`.
- **Surface**: `preferredFormat(adapter)`, `configure(device:format:width:height:presentMode:)`, `currentTexture() throws WgpuError` (a `SurfaceTexture` with `texture` and `suboptimal`), `present()`.
- **SurfaceSource**: `.Xlib(display, window)`, `.Wayland(display, surface)`, `.MetalLayer(layer)`, `.Windows(hinstance, hwnd)`.
- **WgpuError**: `.NoAdapter`, `.NoDevice`, `.Validation`, `.OutOfMemory`, `.Internal`, `.MapFailed`, `.SurfaceOutdated`, `.SurfaceLost` or `.SurfaceTimeout`, each with a message (`message`).

## Raw bindings

`wgpu.raw` mirrors `webgpu.h` names one to one (`WGPUBufferDescriptor`, `wgpuDeviceCreateBuffer`, `WGPUBufferUsage_Storage`, …). Every call needs `unsafe`.

- Descriptors are `FFISafe` structs passed as `&T`, which is C's `const T*`.
- `*CallbackInfo` structs pass by value.
- A callback is an `@export(.C)` function, passed as a `NullableCFunction` in the `*CallbackInfo` field. Its state goes in `userdata1`.

See `src/wgpu.ks` for worked examples.
