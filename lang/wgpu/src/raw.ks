// Raw bindings of wgpu-native's `webgpu.h` (v27) and `wgpu.h`: the C API one
// to one, with no shim. Names match the header so it doubles as the docs.
//
// Structs pass to C by value with the C ABI, `&T` parameters are `const T*`,
// and callbacks are `CFunction`s of `@export(.C)` functions. Only the subset
// the safe `wgpu` module uses is bound; add more the same way.

module wgpu.raw

// ===== Handles =====

public type WGPUInstance = RawPointer
public type WGPUAdapter = RawPointer
public type WGPUDevice = RawPointer
public type WGPUQueue = RawPointer
public type WGPUBuffer = RawPointer
public type WGPUShaderModule = RawPointer
public type WGPUComputePipeline = RawPointer
public type WGPUBindGroupLayout = RawPointer
public type WGPUBindGroup = RawPointer
public type WGPUCommandEncoder = RawPointer
public type WGPUComputePassEncoder = RawPointer
public type WGPUCommandBuffer = RawPointer

// ===== Constants =====

/// `WGPU_STRLEN`: a `WGPUStringView` length meaning "NUL-terminated".
public let WGPU_STRLEN: UInt64 = 18446744073709551615
/// `WGPU_WHOLE_SIZE`: a buffer size meaning "to the end".
public let WGPU_WHOLE_SIZE: UInt64 = 18446744073709551615

public let WGPUCallbackMode_WaitAnyOnly: UInt32 = 1
public let WGPUCallbackMode_AllowProcessEvents: UInt32 = 2
public let WGPUCallbackMode_AllowSpontaneous: UInt32 = 3

public let WGPUSType_ShaderSourceWGSL: UInt32 = 2

public let WGPUFeatureLevel_Core: UInt32 = 2

public let WGPUPowerPreference_Undefined: UInt32 = 0
public let WGPUPowerPreference_LowPower: UInt32 = 1
public let WGPUPowerPreference_HighPerformance: UInt32 = 2

public let WGPUBackendType_Undefined: UInt32 = 0

public let WGPUAdapterType_DiscreteGPU: UInt32 = 1
public let WGPUAdapterType_IntegratedGPU: UInt32 = 2
public let WGPUAdapterType_CPU: UInt32 = 3

public let WGPURequestAdapterStatus_Success: UInt32 = 1
public let WGPURequestDeviceStatus_Success: UInt32 = 1
public let WGPUMapAsyncStatus_Success: UInt32 = 1
public let WGPUPopErrorScopeStatus_Success: UInt32 = 1

public let WGPUErrorType_NoError: UInt32 = 1
public let WGPUErrorType_Validation: UInt32 = 2
public let WGPUErrorType_OutOfMemory: UInt32 = 3
public let WGPUErrorType_Internal: UInt32 = 4

public let WGPUErrorFilter_Validation: UInt32 = 1
public let WGPUErrorFilter_OutOfMemory: UInt32 = 2
public let WGPUErrorFilter_Internal: UInt32 = 3

public let WGPUBufferUsage_MapRead: UInt64 = 1
public let WGPUBufferUsage_MapWrite: UInt64 = 2
public let WGPUBufferUsage_CopySrc: UInt64 = 4
public let WGPUBufferUsage_CopyDst: UInt64 = 8
public let WGPUBufferUsage_Index: UInt64 = 16
public let WGPUBufferUsage_Vertex: UInt64 = 32
public let WGPUBufferUsage_Uniform: UInt64 = 64
public let WGPUBufferUsage_Storage: UInt64 = 128
public let WGPUBufferUsage_Indirect: UInt64 = 256

public let WGPUMapMode_Read: UInt64 = 1
public let WGPUMapMode_Write: UInt64 = 2

// ===== Structs =====

/// A pointer and length into UTF-8 text; `{null, WGPU_STRLEN}` is "no string".
public struct WGPUStringView: FFISafe {
    public var data: Pointer[UInt8]
    public var length: UInt64
}

public struct WGPUChainedStruct: FFISafe {
    public var next: Pointer[WGPUChainedStruct]
    public var sType: UInt32
}

public struct WGPUFuture: FFISafe {
    public var id: UInt64
}

public struct WGPURequestAdapterOptions: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var featureLevel: UInt32
    public var powerPreference: UInt32
    public var forceFallbackAdapter: UInt32
    public var backendType: UInt32
    public var compatibleSurface: RawPointer
}

/// `void (*)(WGPURequestAdapterStatus, WGPUAdapter, WGPUStringView, void*, void*)`
public type WGPURequestAdapterCallback = CFunction[(UInt32, RawPointer, WGPUStringView, RawPointer, RawPointer) -> ()]
/// `void (*)(WGPURequestDeviceStatus, WGPUDevice, WGPUStringView, void*, void*)`
public type WGPURequestDeviceCallback = CFunction[(UInt32, RawPointer, WGPUStringView, RawPointer, RawPointer) -> ()]
/// `void (*)(WGPUMapAsyncStatus, WGPUStringView, void*, void*)`
public type WGPUBufferMapCallback = CFunction[(UInt32, WGPUStringView, RawPointer, RawPointer) -> ()]
/// `void (*)(WGPUPopErrorScopeStatus, WGPUErrorType, WGPUStringView, void*, void*)`
public type WGPUPopErrorScopeCallback = CFunction[(UInt32, UInt32, WGPUStringView, RawPointer, RawPointer) -> ()]
/// `void (*)(WGPUDevice const*, WGPUErrorType, WGPUStringView, void*, void*)`
public type WGPUUncapturedErrorCallback = CFunction[(RawPointer, UInt32, WGPUStringView, RawPointer, RawPointer) -> ()]
/// `void (*)(WGPUDevice const*, WGPUDeviceLostReason, WGPUStringView, void*, void*)`
public type WGPUDeviceLostCallback = CFunction[(RawPointer, UInt32, WGPUStringView, RawPointer, RawPointer) -> ()]

public struct WGPURequestAdapterCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var mode: UInt32
    public var callback: WGPURequestAdapterCallback
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPURequestDeviceCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var mode: UInt32
    public var callback: WGPURequestDeviceCallback
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPUBufferMapCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var mode: UInt32
    public var callback: WGPUBufferMapCallback
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPUPopErrorScopeCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var mode: UInt32
    public var callback: WGPUPopErrorScopeCallback
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPUUncapturedErrorCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var callback: WGPUUncapturedErrorCallback
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPUDeviceLostCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var mode: UInt32
    public var callback: WGPUDeviceLostCallback
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPUQueueDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
}

public struct WGPUDeviceDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
    public var requiredFeatureCount: UInt64
    public var requiredFeatures: Pointer[UInt32]
    public var requiredLimits: RawPointer
    public var defaultQueue: WGPUQueueDescriptor
    public var deviceLostCallbackInfo: WGPUDeviceLostCallbackInfo
    public var uncapturedErrorCallbackInfo: WGPUUncapturedErrorCallbackInfo
}

public struct WGPUAdapterInfo: FFISafe {
    public var nextInChain: RawPointer
    public var vendor: WGPUStringView
    public var architecture: WGPUStringView
    public var device: WGPUStringView
    public var description: WGPUStringView
    public var backendType: UInt32
    public var adapterType: UInt32
    public var vendorID: UInt32
    public var deviceID: UInt32
}

public struct WGPUShaderSourceWGSL: FFISafe {
    public var chain: WGPUChainedStruct
    public var code: WGPUStringView
}

public struct WGPUShaderModuleDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
}

public struct WGPUBufferDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
    public var usage: UInt64
    public var size: UInt64
    public var mappedAtCreation: UInt32
}

public struct WGPUProgrammableStageDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var shaderModule: WGPUShaderModule  // `module` in C
    public var entryPoint: WGPUStringView
    public var constantCount: UInt64
    public var constants: RawPointer
}

public struct WGPUComputePipelineDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
    public var layout: RawPointer
    public var compute: WGPUProgrammableStageDescriptor
}

public struct WGPUBindGroupEntry: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var binding: UInt32
    public var buffer: WGPUBuffer
    public var offset: UInt64
    public var size: UInt64
    public var sampler: RawPointer
    public var textureView: RawPointer
}

public struct WGPUBindGroupDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
    public var layout: WGPUBindGroupLayout
    public var entryCount: UInt64
    public var entries: Pointer[WGPUBindGroupEntry]
}

// ===== Functions =====

@extern(.C, mangleName: "wgpuCreateInstance")
public func wgpuCreateInstance(descriptor: RawPointer) -> WGPUInstance
@extern(.C, mangleName: "wgpuInstanceRequestAdapter")
public func wgpuInstanceRequestAdapter(instance: WGPUInstance, options: &WGPURequestAdapterOptions, callbackInfo: WGPURequestAdapterCallbackInfo) -> WGPUFuture
@extern(.C, mangleName: "wgpuInstanceProcessEvents")
public func wgpuInstanceProcessEvents(instance: WGPUInstance)
@extern(.C, mangleName: "wgpuInstanceRelease")
public func wgpuInstanceRelease(instance: WGPUInstance)

@extern(.C, mangleName: "wgpuAdapterGetInfo")
public func wgpuAdapterGetInfo(adapter: WGPUAdapter, info: &mutating WGPUAdapterInfo) -> UInt32
@extern(.C, mangleName: "wgpuAdapterInfoFreeMembers")
public func wgpuAdapterInfoFreeMembers(adapterInfo: WGPUAdapterInfo)
@extern(.C, mangleName: "wgpuAdapterRequestDevice")
public func wgpuAdapterRequestDevice(adapter: WGPUAdapter, descriptor: &WGPUDeviceDescriptor, callbackInfo: WGPURequestDeviceCallbackInfo) -> WGPUFuture
@extern(.C, mangleName: "wgpuAdapterRelease")
public func wgpuAdapterRelease(adapter: WGPUAdapter)

@extern(.C, mangleName: "wgpuDeviceGetQueue")
public func wgpuDeviceGetQueue(device: WGPUDevice) -> WGPUQueue
@extern(.C, mangleName: "wgpuDeviceCreateShaderModule")
public func wgpuDeviceCreateShaderModule(device: WGPUDevice, descriptor: &WGPUShaderModuleDescriptor) -> WGPUShaderModule
@extern(.C, mangleName: "wgpuDeviceCreateBuffer")
public func wgpuDeviceCreateBuffer(device: WGPUDevice, descriptor: &WGPUBufferDescriptor) -> WGPUBuffer
@extern(.C, mangleName: "wgpuDeviceCreateComputePipeline")
public func wgpuDeviceCreateComputePipeline(device: WGPUDevice, descriptor: &WGPUComputePipelineDescriptor) -> WGPUComputePipeline
@extern(.C, mangleName: "wgpuDeviceCreateBindGroup")
public func wgpuDeviceCreateBindGroup(device: WGPUDevice, descriptor: &WGPUBindGroupDescriptor) -> WGPUBindGroup
@extern(.C, mangleName: "wgpuDeviceCreateCommandEncoder")
public func wgpuDeviceCreateCommandEncoder(device: WGPUDevice, descriptor: RawPointer) -> WGPUCommandEncoder
@extern(.C, mangleName: "wgpuDevicePushErrorScope")
public func wgpuDevicePushErrorScope(device: WGPUDevice, filter: UInt32)
@extern(.C, mangleName: "wgpuDevicePopErrorScope")
public func wgpuDevicePopErrorScope(device: WGPUDevice, callbackInfo: WGPUPopErrorScopeCallbackInfo) -> WGPUFuture
@extern(.C, mangleName: "wgpuDeviceRelease")
public func wgpuDeviceRelease(device: WGPUDevice)
/// From `wgpu.h`: process the device's work, blocking when `wait` is 1.
@extern(.C, mangleName: "wgpuDevicePoll")
public func wgpuDevicePoll(device: WGPUDevice, wait: UInt32, submissionIndex: RawPointer) -> UInt32

@extern(.C, mangleName: "wgpuQueueWriteBuffer")
public func wgpuQueueWriteBuffer(queue: WGPUQueue, buffer: WGPUBuffer, bufferOffset: UInt64, data: RawPointer, size: UInt64)
@extern(.C, mangleName: "wgpuQueueSubmit")
public func wgpuQueueSubmit(queue: WGPUQueue, commandCount: UInt64, commands: &WGPUCommandBuffer)
@extern(.C, mangleName: "wgpuQueueRelease")
public func wgpuQueueRelease(queue: WGPUQueue)

@extern(.C, mangleName: "wgpuShaderModuleRelease")
public func wgpuShaderModuleRelease(shaderModule: WGPUShaderModule)

@extern(.C, mangleName: "wgpuBufferMapAsync")
public func wgpuBufferMapAsync(buffer: WGPUBuffer, mode: UInt64, offset: UInt64, size: UInt64, callbackInfo: WGPUBufferMapCallbackInfo) -> WGPUFuture
@extern(.C, mangleName: "wgpuBufferGetConstMappedRange")
public func wgpuBufferGetConstMappedRange(buffer: WGPUBuffer, offset: UInt64, size: UInt64) -> RawPointer
@extern(.C, mangleName: "wgpuBufferUnmap")
public func wgpuBufferUnmap(buffer: WGPUBuffer)
@extern(.C, mangleName: "wgpuBufferRelease")
public func wgpuBufferRelease(buffer: WGPUBuffer)

@extern(.C, mangleName: "wgpuComputePipelineGetBindGroupLayout")
public func wgpuComputePipelineGetBindGroupLayout(computePipeline: WGPUComputePipeline, groupIndex: UInt32) -> WGPUBindGroupLayout
@extern(.C, mangleName: "wgpuComputePipelineRelease")
public func wgpuComputePipelineRelease(computePipeline: WGPUComputePipeline)
@extern(.C, mangleName: "wgpuBindGroupLayoutRelease")
public func wgpuBindGroupLayoutRelease(bindGroupLayout: WGPUBindGroupLayout)
@extern(.C, mangleName: "wgpuBindGroupRelease")
public func wgpuBindGroupRelease(bindGroup: WGPUBindGroup)

@extern(.C, mangleName: "wgpuCommandEncoderBeginComputePass")
public func wgpuCommandEncoderBeginComputePass(commandEncoder: WGPUCommandEncoder, descriptor: RawPointer) -> WGPUComputePassEncoder
@extern(.C, mangleName: "wgpuCommandEncoderCopyBufferToBuffer")
public func wgpuCommandEncoderCopyBufferToBuffer(commandEncoder: WGPUCommandEncoder, source: WGPUBuffer, sourceOffset: UInt64, destination: WGPUBuffer, destinationOffset: UInt64, size: UInt64)
@extern(.C, mangleName: "wgpuCommandEncoderFinish")
public func wgpuCommandEncoderFinish(commandEncoder: WGPUCommandEncoder, descriptor: RawPointer) -> WGPUCommandBuffer
@extern(.C, mangleName: "wgpuCommandEncoderRelease")
public func wgpuCommandEncoderRelease(commandEncoder: WGPUCommandEncoder)
@extern(.C, mangleName: "wgpuCommandBufferRelease")
public func wgpuCommandBufferRelease(commandBuffer: WGPUCommandBuffer)

@extern(.C, mangleName: "wgpuComputePassEncoderSetPipeline")
public func wgpuComputePassEncoderSetPipeline(computePassEncoder: WGPUComputePassEncoder, pipeline: WGPUComputePipeline)
@extern(.C, mangleName: "wgpuComputePassEncoderSetBindGroup")
public func wgpuComputePassEncoderSetBindGroup(computePassEncoder: WGPUComputePassEncoder, groupIndex: UInt32, group: WGPUBindGroup, dynamicOffsetCount: UInt64, dynamicOffsets: RawPointer)
@extern(.C, mangleName: "wgpuComputePassEncoderDispatchWorkgroups")
public func wgpuComputePassEncoderDispatchWorkgroups(computePassEncoder: WGPUComputePassEncoder, workgroupCountX: UInt32, workgroupCountY: UInt32, workgroupCountZ: UInt32)
@extern(.C, mangleName: "wgpuComputePassEncoderEnd")
public func wgpuComputePassEncoderEnd(computePassEncoder: WGPUComputePassEncoder)
@extern(.C, mangleName: "wgpuComputePassEncoderRelease")
public func wgpuComputePassEncoderRelease(computePassEncoder: WGPUComputePassEncoder)
