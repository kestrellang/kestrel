// A safe wrapper over wgpu-native: owned handles that release themselves,
// blocking adapter/device/readback calls, and errors as `WgpuError`.
//
// Callbacks are `@export(.C)` functions handed to C with `CFunction(f)`; the
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

    /// The error's message from wgpu.
    public var message: String {
        match self {
            .NoAdapter(m) => m.clone(),
            .NoDevice(m) => m.clone(),
            .Validation(m) => m.clone(),
            .OutOfMemory(m) => m.clone(),
            .Internal(m) => m.clone(),
            .MapFailed(m) => m.clone()
        }
    }
}

// ===== Strings =====

/// A view of `s`'s bytes, valid while `s` is.
func view(s: String) -> raw.WGPUStringView {
    raw.WGPUStringView(
        data: RawPointer(raw: s.bytes.asRaw()).cast[UInt8](),
        length: UInt64(from: s.bytes.count)
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
    var count = Int64(from: v.length);
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
    fileprivate var handle: raw.WGPUInstance

    /// Create an instance with the default backends.
    public init() {
        self.handle = unsafe raw.wgpuCreateInstance(RawPointer.nullPointer());
    }

    /// Find an adapter, blocking until wgpu answers. `fallback` asks for a
    /// software adapter, such as lavapipe or WARP.
    public func requestAdapter(power power: PowerPreference = .Default, fallback fallback: Bool = false) -> Adapter throws WgpuError {
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
            compatibleSurface: RawPointer.nullPointer()
        );
        var r = Reply();
        let info = raw.WGPURequestAdapterCallbackInfo(
            nextInChain: Pointer.nullPointer(),
            mode: raw.WGPUCallbackMode_AllowProcessEvents,
            callback: CFunction(onRequest),
            userdata1: Pointer(to: r).asRaw(),
            userdata2: RawPointer.nullPointer()
        );
        unsafe raw.wgpuInstanceRequestAdapter(self.handle, options, info);
        while not r.done {
            unsafe raw.wgpuInstanceProcessEvents(self.handle);
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
    fileprivate var handle: raw.WGPUAdapter

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
    public func requestDevice(label label: String = "") -> Device throws WgpuError {
        let descriptor = raw.WGPUDeviceDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            requiredFeatureCount: 0,
            requiredFeatures: Pointer.nullPointer(),
            requiredLimits: RawPointer.nullPointer(),
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
            callback: CFunction(onRequest),
            userdata1: Pointer(to: r).asRaw(),
            userdata2: RawPointer.nullPointer()
        );
        unsafe raw.wgpuAdapterRequestDevice(self.handle, descriptor, info);
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

    fileprivate var bits: UInt64 {
        match self {
            .MapRead => raw.WGPUBufferUsage_MapRead,
            .MapWrite => raw.WGPUBufferUsage_MapWrite,
            .CopySrc => raw.WGPUBufferUsage_CopySrc,
            .CopyDst => raw.WGPUBufferUsage_CopyDst,
            .Index => raw.WGPUBufferUsage_Index,
            .Vertex => raw.WGPUBufferUsage_Vertex,
            .Uniform => raw.WGPUBufferUsage_Uniform,
            .Storage => raw.WGPUBufferUsage_Storage,
            .Indirect => raw.WGPUBufferUsage_Indirect
        }
    }
}

/// A logical GPU and its queue: creates resources and runs work.
public struct Device: not Copyable {
    fileprivate var handle: raw.WGPUDevice
    fileprivate var queue: raw.WGPUQueue

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
            size: UInt64(from: size),
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
    public func createComputePipeline(shader shader: ShaderModule, entryPoint entryPoint: String, label label: String = "") -> ComputePipeline {
        let descriptor = raw.WGPUComputePipelineDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            layout: RawPointer.nullPointer(),
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

    /// Bind resources to group `group` of `pipeline`, as
    /// `buffer.binding(n)` gives them.
    public func createBindGroup(pipeline pipeline: ComputePipeline, group group: Int64, entries bindings: [Binding], label label: String = "") -> BindGroup {
        var entries: [raw.WGPUBindGroupEntry] = [];
        for b in bindings {
            entries.append(b.entry);
        }
        let layout = unsafe raw.wgpuComputePipelineGetBindGroupLayout(pipeline.handle, UInt32(from: group));
        let descriptor = raw.WGPUBindGroupDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            layout: layout,
            entryCount: UInt64(from: entries.count),
            entries: entries.asPointer()
        );
        let group = unsafe raw.wgpuDeviceCreateBindGroup(self.handle, descriptor);
        unsafe raw.wgpuBindGroupLayoutRelease(layout);
        BindGroup(handle: group)
    }

    /// Start recording commands.
    public func createCommandEncoder() -> CommandEncoder {
        CommandEncoder(handle: unsafe raw.wgpuDeviceCreateCommandEncoder(self.handle, RawPointer.nullPointer()))
    }

    /// Copy `data` into `buffer` at byte `offset`.
    public func write[T](buffer: Buffer, data data: [T], offset offset: Int64 = 0) where T: FFISafe {
        let size = Layout.array[T](data.count).size;
        unsafe raw.wgpuQueueWriteBuffer(self.queue, buffer.handle, UInt64(from: offset), data.asPointer().asRaw(), UInt64(from: size));
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
        let size = UInt64(from: buffer.size);
        let info = raw.WGPUBufferMapCallbackInfo(
            nextInChain: Pointer.nullPointer(),
            mode: raw.WGPUCallbackMode_AllowProcessEvents,
            callback: CFunction(onMapped),
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
            callback: CFunction(onPopError),
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
    fileprivate var handle: raw.WGPUBuffer
    /// The size in bytes.
    public let size: Int64

    /// The whole buffer at `@binding(index)`, for `Device.createBindGroup`.
    public func binding(index: Int64) -> Binding {
        Binding(entry: raw.WGPUBindGroupEntry(
            nextInChain: Pointer.nullPointer(),
            binding: UInt32(from: index),
            buffer: self.handle,
            offset: 0,
            size: UInt64(from: self.size),
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
    fileprivate var entry: raw.WGPUBindGroupEntry
}

/// A compiled shader.
public struct ShaderModule: not Copyable {
    fileprivate var handle: raw.WGPUShaderModule

    deinit {
        unsafe raw.wgpuShaderModuleRelease(self.handle);
    }
}

/// A compute shader ready to dispatch.
public struct ComputePipeline: not Copyable {
    fileprivate var handle: raw.WGPUComputePipeline

    deinit {
        unsafe raw.wgpuComputePipelineRelease(self.handle);
    }
}

/// Resources bound for a pipeline.
public struct BindGroup: not Copyable {
    fileprivate var handle: raw.WGPUBindGroup

    deinit {
        unsafe raw.wgpuBindGroupRelease(self.handle);
    }
}

// ===== Commands =====

/// Records GPU commands.
public struct CommandEncoder: not Copyable {
    fileprivate var handle: raw.WGPUCommandEncoder

    /// Record a compute pass: `body` sets pipelines and dispatches.
    public func computePass(body: (ComputePass) -> ()) {
        let pass = ComputePass(handle: unsafe raw.wgpuCommandEncoderBeginComputePass(self.handle, RawPointer.nullPointer()));
        body(pass);
        unsafe raw.wgpuComputePassEncoderEnd(pass.handle);
    }

    /// Copy `size` bytes from `source` to `destination`.
    public func copy(from source: Buffer, to destination: Buffer, size size: Int64) {
        unsafe raw.wgpuCommandEncoderCopyBufferToBuffer(self.handle, source.handle, 0, destination.handle, 0, UInt64(from: size));
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
    fileprivate var handle: raw.WGPUComputePassEncoder

    /// Use `pipeline` for the next dispatches.
    public func setPipeline(pipeline: ComputePipeline) {
        unsafe raw.wgpuComputePassEncoderSetPipeline(self.handle, pipeline.handle);
    }

    /// Bind `bindGroup` at `index`.
    public func setBindGroup(index: Int64, bindGroup: BindGroup) {
        unsafe raw.wgpuComputePassEncoderSetBindGroup(self.handle, UInt32(from: index), bindGroup.handle, 0, RawPointer.nullPointer());
    }

    /// Run `x * y * z` workgroups.
    public func dispatch(x: Int64, y y: Int64 = 1, z z: Int64 = 1) {
        unsafe raw.wgpuComputePassEncoderDispatchWorkgroups(self.handle, UInt32(from: x), UInt32(from: y), UInt32(from: z));
    }

    deinit {
        unsafe raw.wgpuComputePassEncoderRelease(self.handle);
    }
}

/// Recorded commands, ready to submit.
public struct CommandBuffer: not Copyable {
    fileprivate var handle: raw.WGPUCommandBuffer

    deinit {
        unsafe raw.wgpuCommandBufferRelease(self.handle);
    }
}
