// A safe wrapper over wgpu-native: owned handles that release themselves,
// blocking adapter/device/readback calls, and errors as `WgpuError`. This
// file has the instance, adapter, device, buffers and compute; `render.ks`
// has textures and rendering, `surface.ks` window surfaces.
//
// Callbacks are `@export(.C)` functions handed to C as C function values; the
// state they fill travels through the C API's `userdata1` pointer.

module wgpu

import wgpu.raw

// ===== Errors =====

/// Why a wgpu call failed.
public enum WgpuError {
    /// No adapter matched the request.
    case NoAdapter(String)
    /// The adapter could not create a device.
    case NoDevice(String)
    /// The call broke a WebGPU validation rule.
    case Validation(String)
    /// The GPU ran out of memory.
    case OutOfMemory(String)
    /// An internal wgpu error.
    case Internal(String)
    /// Mapping a buffer for reading failed.
    case MapFailed(String)
    /// The surface changed size or settings: configure it again.
    case SurfaceOutdated(String)
    /// The surface is gone: create a new one.
    case SurfaceLost(String)
    /// No frame became available in time; try again.
    case SurfaceTimeout(String)

    /// The error's message from wgpu.
    public var message: String {
        match self {
            .NoAdapter(m) => m.clone(),
            .NoDevice(m) => m.clone(),
            .Validation(m) => m.clone(),
            .OutOfMemory(m) => m.clone(),
            .Internal(m) => m.clone(),
            .MapFailed(m) => m.clone(),
            .SurfaceOutdated(m) => m.clone(),
            .SurfaceLost(m) => m.clone(),
            .SurfaceTimeout(m) => m.clone()
        }
    }
}

// ===== Strings =====

/// A view of `s`'s bytes, valid while `s` is.
func view(s: String) -> raw.WGPUStringView {
    raw.WGPUStringView(
        data: RawPointer(raw: s.bytes.asRaw()).cast[UInt8](),
        length: UInt64(exactly: s.bytes.count)!
    )
}

/// The "no string" view.
func noString() -> raw.WGPUStringView {
    raw.WGPUStringView(data: Pointer.nullPointer(), length: raw.WGPU_STRLEN)
}

/// A copy of the text `v` points to.
func copyString(v: raw.WGPUStringView) -> String {
    if v.data.isNull {
        return String()
    }
    var count = Int64(exactly: v.length)!;
    if v.length == raw.WGPU_STRLEN {
        count = CString(raw: v.data).length;
    }
    let bytes = unsafe UnsafeSlice(pointer: v.data, count: count);
    String(fromUtf8Lossy: bytes)
}

// ===== Callbacks =====

/// What a wgpu callback reports, filled in through `userdata1`.
struct Reply {
    var done: Bool
    var status: UInt32
    var code: UInt32
    var handle: RawPointer
    var message: String

    init() {
        self.done = false;
        self.status = 0;
        self.code = 0;
        self.handle = RawPointer.nullPointer();
        self.message = String();
    }
}

/// Fill in the `Reply` at `userdata`.
func reply(userdata: RawPointer, status: UInt32, code: UInt32, handle: RawPointer, message: raw.WGPUStringView) {
    let r = userdata.cast[Reply]();
    unsafe {
        r.mutatingValue.done = true;
        r.mutatingValue.status = status;
        r.mutatingValue.code = code;
        r.mutatingValue.handle = handle;
        r.mutatingValue.message = copyString(message);
    }
}

// `WGPURequestAdapterCallback` and `WGPURequestDeviceCallback`.
@export(.C, mangleName: "kestrel_wgpu_on_request")
func onRequest(status: UInt32, handle: RawPointer, message: raw.WGPUStringView, userdata1: RawPointer, userdata2: RawPointer) {
    reply(userdata1, status, 0, handle, message)
}

// `WGPUBufferMapCallback`.
@export(.C, mangleName: "kestrel_wgpu_on_mapped")
func onMapped(status: UInt32, message: raw.WGPUStringView, userdata1: RawPointer, userdata2: RawPointer) {
    reply(userdata1, status, 0, RawPointer.nullPointer(), message)
}

// `WGPUPopErrorScopeCallback`.
@export(.C, mangleName: "kestrel_wgpu_on_pop_error")
func onPopError(status: UInt32, errorType: UInt32, message: raw.WGPUStringView, userdata1: RawPointer, userdata2: RawPointer) {
    reply(userdata1, status, errorType, RawPointer.nullPointer(), message)
}

// ===== Instance =====

/// Which adapter to prefer.
public enum PowerPreference {
    case Default
    case LowPower
    case HighPerformance
}

/// The entry point to wgpu: finds adapters.
public struct Instance: not Copyable {
    var handle: raw.WGPUInstance

    /// Create an instance with the default backends.
    public init() {
        self.handle = unsafe raw.wgpuCreateInstance(RawPointer.nullPointer());
    }

    /// Find an adapter, blocking until wgpu answers. `fallback` asks for a
    /// software adapter, such as lavapipe or WARP.
    public func requestAdapter(power power: PowerPreference = .Default, fallback fallback: Bool = false) -> Adapter throws WgpuError {
        self.findAdapter(power, fallback, RawPointer.nullPointer())
    }

    /// Find an adapter that can present to `surface`.
    public func requestAdapter(compatibleWith surface: Surface, power power: PowerPreference = .Default, fallback fallback: Bool = false) -> Adapter throws WgpuError {
        self.findAdapter(power, fallback, surface.handle)
    }

    func findAdapter(power: PowerPreference, fallback: Bool, surface: RawPointer) -> Adapter throws WgpuError {
        let options = raw.WGPURequestAdapterOptions(
            nextInChain: Pointer.nullPointer(),
            featureLevel: raw.WGPUFeatureLevel_Core,
            powerPreference: match power {
                .Default => raw.WGPUPowerPreference_Undefined,
                .LowPower => raw.WGPUPowerPreference_LowPower,
                .HighPerformance => raw.WGPUPowerPreference_HighPerformance
            },
            forceFallbackAdapter: if fallback { 1 } else { 0 },
            backendType: raw.WGPUBackendType_Undefined,
            compatibleSurface: surface
        );
        var r = Reply();
        let info = raw.WGPURequestAdapterCallbackInfo(
            nextInChain: Pointer.nullPointer(),
            mode: raw.WGPUCallbackMode_AllowProcessEvents,
            callback: NullableCFunction(onRequest),
            userdata1: Pointer(to: r).asRaw(),
            userdata2: RawPointer.nullPointer()
        );
        unsafe raw.wgpuInstanceRequestAdapter(self.handle, options, info);
        while not r.done {
            unsafe raw.wgpuInstanceProcessEvents(self.handle);
            let _ = raw.yieldToHost();
        }
        if r.status != raw.WGPURequestAdapterStatus_Success {
            throw WgpuError.NoAdapter(r.message.clone());
        }
        Adapter(handle: r.handle)
    }

    deinit {
        unsafe raw.wgpuInstanceRelease(self.handle);
    }
}

// ===== Adapter =====

/// What kind of device an adapter drives.
public enum AdapterType {
    case DiscreteGPU
    case IntegratedGPU
    case CPU
    case Unknown
}

/// An adapter's name and kind.
public struct AdapterInfo {
    public var name: String
    public var vendor: String
    public var description: String
    public var adapterType: AdapterType
}

/// A physical GPU (or a software one): creates devices.
public struct Adapter: not Copyable {
    var handle: raw.WGPUAdapter

    /// The adapter's name and kind.
    public var info: AdapterInfo {
        var info = raw.WGPUAdapterInfo(
            nextInChain: RawPointer.nullPointer(),
            vendor: noString(),
            architecture: noString(),
            device: noString(),
            description: noString(),
            backendType: 0,
            adapterType: 0,
            vendorID: 0,
            deviceID: 0
        );
        unsafe raw.wgpuAdapterGetInfo(self.handle, info);
        let result = AdapterInfo(
            name: copyString(info.device),
            vendor: copyString(info.vendor),
            description: copyString(info.description),
            adapterType: if info.adapterType == raw.WGPUAdapterType_DiscreteGPU {
                .DiscreteGPU
            } else if info.adapterType == raw.WGPUAdapterType_IntegratedGPU {
                .IntegratedGPU
            } else if info.adapterType == raw.WGPUAdapterType_CPU {
                .CPU
            } else {
                .Unknown
            }
        );
        unsafe raw.wgpuAdapterInfoFreeMembers(info);
        result
    }

    /// Create a device with the default limits, blocking until wgpu answers.
    /// A device with `features` (check `supports(feature)` first). A
    /// `maxPushConstantSize` above 0 also asks for `Feature.PushConstants`
    /// with room for that many bytes.
    public func requestDevice(label label: String = "", features features: [Feature] = [], maxPushConstantSize maxPushConstantSize: Int64 = 0) -> Device throws WgpuError {
        var featureList: [UInt32] = [];
        var hasPushConstants = false;
        for f in features {
            featureList.append(f.cValue);
            hasPushConstants = hasPushConstants or f.cValue == raw.WGPUNativeFeature_PushConstants;
        }
        if maxPushConstantSize > 0 and not hasPushConstants {
            featureList.append(raw.WGPUNativeFeature_PushConstants);
        }
        // Push constants are a wgpu-native limit, chained onto the standard
        // ones (all left at their defaults).
        var nativeLimits = raw.WGPUNativeLimits(
            chain: raw.WGPUChainedStruct(next: Pointer.nullPointer(), sType: raw.WGPUSType_NativeLimits),
            maxPushConstantSize: UInt32(exactly: maxPushConstantSize)!,
            maxNonSamplerBindings: raw.WGPU_U32_UNDEFINED
        );
        var limits = undefinedLimits(Pointer(to: nativeLimits).cast[raw.WGPUChainedStruct]());
        let descriptor = raw.WGPUDeviceDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            requiredFeatureCount: UInt64(exactly: featureList.count)!,
            requiredFeatures: featureList.asPointer(),
            requiredLimits: if maxPushConstantSize > 0 { Pointer(to: limits).asRaw() } else { RawPointer.nullPointer() },
            defaultQueue: raw.WGPUQueueDescriptor(nextInChain: Pointer.nullPointer(), label: noString()),
            deviceLostCallbackInfo: raw.WGPUDeviceLostCallbackInfo(
                nextInChain: Pointer.nullPointer(),
                mode: raw.WGPUCallbackMode_AllowProcessEvents,
                callback: null,
                userdata1: RawPointer.nullPointer(),
                userdata2: RawPointer.nullPointer()
            ),
            uncapturedErrorCallbackInfo: raw.WGPUUncapturedErrorCallbackInfo(
                nextInChain: Pointer.nullPointer(),
                callback: null,
                userdata1: RawPointer.nullPointer(),
                userdata2: RawPointer.nullPointer()
            )
        );
        var r = Reply();
        let info = raw.WGPURequestDeviceCallbackInfo(
            nextInChain: Pointer.nullPointer(),
            mode: raw.WGPUCallbackMode_AllowSpontaneous,
            callback: NullableCFunction(onRequest),
            userdata1: Pointer(to: r).asRaw(),
            userdata2: RawPointer.nullPointer()
        );
        unsafe raw.wgpuAdapterRequestDevice(self.handle, descriptor, info);
        // wgpu-native answers before returning; a browser on a later turn.
        while not r.done and raw.yieldToHost() {}
        if not r.done or r.status != raw.WGPURequestDeviceStatus_Success {
            throw WgpuError.NoDevice(r.message.clone());
        }
        let queue = unsafe raw.wgpuDeviceGetQueue(r.handle);
        Device(handle: r.handle, queue: queue)
    }

    deinit {
        unsafe raw.wgpuAdapterRelease(self.handle);
    }
}

// ===== Device =====

/// What a buffer may be used for.
public enum BufferUsage {
    case MapRead
    case MapWrite
    case CopySrc
    case CopyDst
    case Index
    case Vertex
    case Uniform
    case Storage
    case Indirect
    /// A destination for `CommandEncoder.resolve(querySet)`.
    case QueryResolve

    var bits: UInt64 {
        match self {
            .MapRead => raw.WGPUBufferUsage_MapRead,
            .MapWrite => raw.WGPUBufferUsage_MapWrite,
            .CopySrc => raw.WGPUBufferUsage_CopySrc,
            .CopyDst => raw.WGPUBufferUsage_CopyDst,
            .Index => raw.WGPUBufferUsage_Index,
            .Vertex => raw.WGPUBufferUsage_Vertex,
            .Uniform => raw.WGPUBufferUsage_Uniform,
            .Storage => raw.WGPUBufferUsage_Storage,
            .Indirect => raw.WGPUBufferUsage_Indirect,
            .QueryResolve => raw.WGPUBufferUsage_QueryResolve
        }
    }
}

/// A logical GPU and its queue: creates resources and runs work.
public struct Device: not Copyable {
    var handle: raw.WGPUDevice
    var queue: raw.WGPUQueue

    /// Create a buffer of `size` bytes.
    public func createBuffer(size size: Int64, usage usage: [BufferUsage], label label: String = "") -> Buffer {
        var bits: UInt64 = 0;
        for u in usage {
            bits = bits | u.bits;
        }
        let descriptor = raw.WGPUBufferDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            usage: bits,
            size: UInt64(exactly: size)!,
            mappedAtCreation: 0
        );
        Buffer(handle: unsafe raw.wgpuDeviceCreateBuffer(self.handle, descriptor), size: size)
    }

    /// Compile a WGSL shader.
    public func createShaderModule(wgsl wgsl: String, label label: String = "") -> ShaderModule {
        var source = raw.WGPUShaderSourceWGSL(
            chain: raw.WGPUChainedStruct(next: Pointer.nullPointer(), sType: raw.WGPUSType_ShaderSourceWGSL),
            code: view(wgsl)
        );
        let descriptor = raw.WGPUShaderModuleDescriptor(
            nextInChain: Pointer(to: source).cast[raw.WGPUChainedStruct](),
            label: view(label)
        );
        ShaderModule(handle: unsafe raw.wgpuDeviceCreateShaderModule(self.handle, descriptor))
    }

    /// Create a compute pipeline running `entryPoint` of `shader`, with its
    /// bind group layouts inferred from the shader.
    /// Bind group layouts come from `layout`, or are inferred from the
    /// shader when it is `None`.
    public func createComputePipeline(shader shader: ShaderModule, entryPoint entryPoint: String, layout layout: PipelineLayout? = .None, label label: String = "") -> ComputePipeline {
        let descriptor = raw.WGPUComputePipelineDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            layout: match layout {
                .Some(l) => l.handle,
                .None => RawPointer.nullPointer()
            },
            compute: raw.WGPUProgrammableStageDescriptor(
                nextInChain: Pointer.nullPointer(),
                shaderModule: shader.handle,
                entryPoint: view(entryPoint),
                constantCount: 0,
                constants: RawPointer.nullPointer()
            )
        );
        ComputePipeline(handle: unsafe raw.wgpuDeviceCreateComputePipeline(self.handle, descriptor))
    }

    /// Bind resources to `layout`, as `buffer.binding(n)`,
    /// `view.binding(n)` and `sampler.binding(n)` give them. Get the layout
    /// from a pipeline with `bindGroupLayout(group)`.
    public func createBindGroup(layout layout: BindGroupLayout, entries bindings: [Binding], label label: String = "") -> BindGroup {
        var entries: [raw.WGPUBindGroupEntry] = [];
        for b in bindings {
            entries.append(b.entry);
        }
        let descriptor = raw.WGPUBindGroupDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            layout: layout.handle,
            entryCount: UInt64(exactly: entries.count)!,
            entries: entries.asPointer()
        );
        BindGroup(handle: unsafe raw.wgpuDeviceCreateBindGroup(self.handle, descriptor))
    }

    /// Start recording commands.
    public func createCommandEncoder() -> CommandEncoder {
        CommandEncoder(handle: unsafe raw.wgpuDeviceCreateCommandEncoder(self.handle, RawPointer.nullPointer()))
    }

    /// A buffer holding `data`, sized to fit it.
    public func createBuffer[T](data data: [T], usage usage: [BufferUsage], label label: String = "") -> Buffer where T: FFISafe {
        var all = usage.clone();
        all.append(.CopyDst);
        let buffer = self.createBuffer(size: Layout.array[T](data.count).size, usage: all, label: label);
        self.write(buffer, data: data);
        buffer
    }

    /// Copy `data` into `buffer` at byte `offset`.
    public func write[T](buffer: Buffer, data data: [T], offset offset: Int64 = 0) where T: FFISafe {
        let size = Layout.array[T](data.count).size;
        unsafe raw.wgpuQueueWriteBuffer(self.queue, buffer.handle, UInt64(exactly: offset)!, data.asPointer().asRaw(), UInt64(exactly: size)!);
    }

    /// Copy one value, such as a uniform struct, into `buffer` at byte
    /// `offset`.
    public func write[T](buffer: Buffer, value value: T, offset offset: Int64 = 0) where T: FFISafe {
        let size = Layout.of[T]().size;
        unsafe raw.wgpuQueueWriteBuffer(self.queue, buffer.handle, UInt64(exactly: offset)!, Pointer(to: value).asRaw(), UInt64(exactly: size)!);
    }

    /// Run recorded commands.
    public func submit(consuming commands: CommandBuffer) {
        let handle = commands.handle;
        unsafe raw.wgpuQueueSubmit(self.queue, 1, handle);
    }

    /// Wait for submitted work to finish.
    public func wait() {
        unsafe raw.wgpuDevicePoll(self.handle, 1, RawPointer.nullPointer());
    }

    /// Read `buffer` (made with `.MapRead`) back as `T`s, waiting for the GPU.
    public func read[T](buffer: Buffer) -> [T] throws WgpuError where T: FFISafe {
        var r = Reply();
        let size = UInt64(exactly: buffer.size)!;
        let info = raw.WGPUBufferMapCallbackInfo(
            nextInChain: Pointer.nullPointer(),
            mode: raw.WGPUCallbackMode_AllowProcessEvents,
            callback: NullableCFunction(onMapped),
            userdata1: Pointer(to: r).asRaw(),
            userdata2: RawPointer.nullPointer()
        );
        unsafe raw.wgpuBufferMapAsync(buffer.handle, raw.WGPUMapMode_Read, 0, size, info);
        while not r.done {
            unsafe raw.wgpuDevicePoll(self.handle, 1, RawPointer.nullPointer());
        }
        if r.status != raw.WGPUMapAsyncStatus_Success {
            throw WgpuError.MapFailed(r.message.clone());
        }
        let mapped = unsafe raw.wgpuBufferGetConstMappedRange(buffer.handle, 0, size).cast[T]();
        let count = buffer.size / Layout.of[T]().size;
        var out: [T] = [];
        var i = 0;
        while i < count {
            out.append(unsafe mapped.offset(by: i).read());
            i = i + 1;
        }
        unsafe raw.wgpuBufferUnmap(buffer.handle);
        out
    }

    /// Run `body`, then report the first validation error it caused, if any.
    public func catchErrors(body: () -> ()) -> () throws WgpuError {
        unsafe raw.wgpuDevicePushErrorScope(self.handle, raw.WGPUErrorFilter_Validation);
        body();
        var r = Reply();
        let info = raw.WGPUPopErrorScopeCallbackInfo(
            nextInChain: Pointer.nullPointer(),
            mode: raw.WGPUCallbackMode_AllowSpontaneous,
            callback: NullableCFunction(onPopError),
            userdata1: Pointer(to: r).asRaw(),
            userdata2: RawPointer.nullPointer()
        );
        unsafe raw.wgpuDevicePopErrorScope(self.handle, info);
        while not r.done {
            unsafe raw.wgpuDevicePoll(self.handle, 1, RawPointer.nullPointer());
        }
        if r.code == raw.WGPUErrorType_Validation {
            throw WgpuError.Validation(r.message.clone());
        }
        if r.code == raw.WGPUErrorType_OutOfMemory {
            throw WgpuError.OutOfMemory(r.message.clone());
        }
        if r.code == raw.WGPUErrorType_Internal {
            throw WgpuError.Internal(r.message.clone());
        }
        .Ok(())
    }

    deinit {
        unsafe raw.wgpuQueueRelease(self.queue);
        unsafe raw.wgpuDeviceRelease(self.handle);
    }
}

// ===== Resources =====

/// GPU memory.
public struct Buffer: not Copyable {
    var handle: raw.WGPUBuffer
    /// The size in bytes.
    public let size: Int64

    /// The buffer at `@binding(index)`, for `Device.createBindGroup`: from
    /// byte `offset`, for `size` bytes or the rest of it. A dynamic binding's
    /// `size` is one element's; `setBindGroup` picks which.
    public func binding(index: Int64, offset offset: Int64 = 0, size size: Int64? = .None) -> Binding {
        Binding(entry: raw.WGPUBindGroupEntry(
            nextInChain: Pointer.nullPointer(),
            binding: UInt32(exactly: index)!,
            buffer: self.handle,
            offset: UInt64(exactly: offset)!,
            size: UInt64(exactly: size ?? (self.size - offset))!,
            sampler: RawPointer.nullPointer(),
            textureView: RawPointer.nullPointer()
        ))
    }

    deinit {
        unsafe raw.wgpuBufferRelease(self.handle);
    }
}

/// One resource of a bind group. It doesn't own the resource, which must
/// outlive the bind group's creation.
public struct Binding {
    var entry: raw.WGPUBindGroupEntry
}

/// A compiled shader.
public struct ShaderModule: not Copyable {
    var handle: raw.WGPUShaderModule

    deinit {
        unsafe raw.wgpuShaderModuleRelease(self.handle);
    }
}

/// A compute shader ready to dispatch.
public struct ComputePipeline: not Copyable {
    var handle: raw.WGPUComputePipeline

    /// The layout of bind group `group`, inferred from the shader.
    public func bindGroupLayout(group: Int64) -> BindGroupLayout {
        BindGroupLayout(handle: unsafe raw.wgpuComputePipelineGetBindGroupLayout(self.handle, UInt32(exactly: group)!))
    }

    deinit {
        unsafe raw.wgpuComputePipelineRelease(self.handle);
    }
}

/// What a bind group holds, from a pipeline's `bindGroupLayout(group)`.
public struct BindGroupLayout: not Copyable {
    var handle: raw.WGPUBindGroupLayout

    deinit {
        unsafe raw.wgpuBindGroupLayoutRelease(self.handle);
    }
}

/// Resources bound for a pipeline.
public struct BindGroup: not Copyable {
    var handle: raw.WGPUBindGroup

    deinit {
        unsafe raw.wgpuBindGroupRelease(self.handle);
    }
}

// ===== Commands =====

/// Records GPU commands.
public struct CommandEncoder: not Copyable {
    var handle: raw.WGPUCommandEncoder

    /// Record a compute pass: `body` sets pipelines and dispatches.
    public func computePass(body: (ComputePass) -> ()) {
        let pass = ComputePass(handle: unsafe raw.wgpuCommandEncoderBeginComputePass(self.handle, RawPointer.nullPointer()));
        body(pass);
        unsafe raw.wgpuComputePassEncoderEnd(pass.handle);
    }

    /// Record a compute pass that writes `timestamps` at its start and end.
    public func computePass(timestamps timestamps: PassTimestamps, body: (ComputePass) -> ()) {
        var writes = timestamps.cValue;
        let descriptor = raw.WGPUComputePassDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: noString(),
            timestampWrites: Pointer(to: writes)
        );
        let pass = ComputePass(handle: unsafe raw.wgpuCommandEncoderBeginComputePassWith(self.handle, descriptor));
        body(pass);
        unsafe raw.wgpuComputePassEncoderEnd(pass.handle);
    }

    /// Copy `size` bytes from `source` to `destination`.
    public func copy(from source: Buffer, to destination: Buffer, size size: Int64) {
        unsafe raw.wgpuCommandEncoderCopyBufferToBuffer(self.handle, source.handle, 0, destination.handle, 0, UInt64(exactly: size)!);
    }

    /// Stop recording.
    public consuming func finish() -> CommandBuffer {
        CommandBuffer(handle: unsafe raw.wgpuCommandEncoderFinish(self.handle, RawPointer.nullPointer()))
    }

    deinit {
        unsafe raw.wgpuCommandEncoderRelease(self.handle);
    }
}

/// A compute pass being recorded.
public struct ComputePass: not Copyable {
    var handle: raw.WGPUComputePassEncoder

    /// Use `pipeline` for the next dispatches.
    public func setPipeline(pipeline: ComputePipeline) {
        unsafe raw.wgpuComputePassEncoderSetPipeline(self.handle, pipeline.handle);
    }

    /// Bind `bindGroup` at `index`, with one byte offset per dynamic
    /// binding, in binding order.
    public func setBindGroup(index: Int64, bindGroup: BindGroup, dynamicOffsets dynamicOffsets: [Int64] = []) {
        let offsets = dynamicOffsetArray(dynamicOffsets);
        unsafe raw.wgpuComputePassEncoderSetBindGroup(self.handle, UInt32(exactly: index)!, bindGroup.handle, UInt64(exactly: offsets.count)!, offsets.asPointer().asRaw());
    }

    /// Set push constants (see `Feature.PushConstants`) from `value`,
    /// starting `offset` bytes in.
    public func setPushConstants[T](value value: T, offset offset: Int64 = 0) where T: FFISafe {
        let size = Layout.of[T]().size;
        unsafe raw.wgpuComputePassEncoderSetPushConstants(self.handle, UInt32(exactly: offset)!, UInt32(exactly: size)!, Pointer(to: value).asRaw());
    }

    /// Run `x * y * z` workgroups.
    public func dispatch(x: Int64, y y: Int64 = 1, z z: Int64 = 1) {
        unsafe raw.wgpuComputePassEncoderDispatchWorkgroups(self.handle, UInt32(exactly: x)!, UInt32(exactly: y)!, UInt32(exactly: z)!);
    }

    /// Run the workgroup counts (three `UInt32`s) read from `buffer`, made
    /// with `.Indirect`, at `offset`.
    public func dispatchIndirect(buffer: Buffer, offset offset: Int64 = 0) {
        unsafe raw.wgpuComputePassEncoderDispatchWorkgroupsIndirect(self.handle, buffer.handle, UInt64(exactly: offset)!);
    }

    deinit {
        unsafe raw.wgpuComputePassEncoderRelease(self.handle);
    }
}

/// Recorded commands, ready to submit.
public struct CommandBuffer: not Copyable {
    var handle: raw.WGPUCommandBuffer

    deinit {
        unsafe raw.wgpuCommandBufferRelease(self.handle);
    }
}
