// Raw bindings of wgpu-native's `webgpu.h` (v27) and `wgpu.h`: the C API one
// to one, with no shim. Names match the header so it doubles as the docs.
//
// Structs pass to C by value with the C ABI, `&T` parameters are `const T*`,
// and callbacks are C function types (`@extern(.C) (…) -> …`) filled with
// `@export(.C)` functions. The callback fields C allows to be NULL are
// `NullableCFunction`s. Only the subset the safe `wgpu` module uses is bound;
// add more the same way.

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
public type WGPUTexture = RawPointer
public type WGPUTextureView = RawPointer
public type WGPUSampler = RawPointer
public type WGPURenderPipeline = RawPointer
public type WGPURenderPassEncoder = RawPointer
public type WGPUSurface = RawPointer

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

/// `WGPU_DEPTH_SLICE_UNDEFINED`, `WGPU_MIP_LEVEL_COUNT_UNDEFINED`,
/// `WGPU_ARRAY_LAYER_COUNT_UNDEFINED` and `WGPU_COPY_STRIDE_UNDEFINED`.
public let WGPU_U32_UNDEFINED: UInt32 = 4294967295

public let WGPUSType_SurfaceSourceMetalLayer: UInt32 = 4
public let WGPUSType_SurfaceSourceWindowsHWND: UInt32 = 5
public let WGPUSType_SurfaceSourceXlibWindow: UInt32 = 6
public let WGPUSType_SurfaceSourceWaylandSurface: UInt32 = 7

public let WGPUStatus_Success: UInt32 = 1

public let WGPUTextureUsage_CopySrc: UInt64 = 1
public let WGPUTextureUsage_CopyDst: UInt64 = 2
public let WGPUTextureUsage_TextureBinding: UInt64 = 4
public let WGPUTextureUsage_StorageBinding: UInt64 = 8
public let WGPUTextureUsage_RenderAttachment: UInt64 = 16

public let WGPUTextureFormat_Undefined: UInt32 = 0
public let WGPUTextureFormat_R8Unorm: UInt32 = 1
public let WGPUTextureFormat_R32Float: UInt32 = 12
public let WGPUTextureFormat_RGBA8Unorm: UInt32 = 18
public let WGPUTextureFormat_RGBA8UnormSrgb: UInt32 = 19
public let WGPUTextureFormat_BGRA8Unorm: UInt32 = 23
public let WGPUTextureFormat_BGRA8UnormSrgb: UInt32 = 24
public let WGPUTextureFormat_RGBA16Float: UInt32 = 34
public let WGPUTextureFormat_RGBA32Float: UInt32 = 35
public let WGPUTextureFormat_Depth16Unorm: UInt32 = 39
public let WGPUTextureFormat_Depth24Plus: UInt32 = 40
public let WGPUTextureFormat_Depth24PlusStencil8: UInt32 = 41
public let WGPUTextureFormat_Depth32Float: UInt32 = 42

public let WGPUTextureDimension_2D: UInt32 = 2
public let WGPUTextureViewDimension_Undefined: UInt32 = 0
public let WGPUTextureAspect_All: UInt32 = 1

public let WGPUVertexStepMode_Vertex: UInt32 = 2
public let WGPUVertexStepMode_Instance: UInt32 = 3

public let WGPUVertexFormat_Uint8x4: UInt32 = 3
public let WGPUVertexFormat_Unorm8x4: UInt32 = 9
public let WGPUVertexFormat_Float32: UInt32 = 28
public let WGPUVertexFormat_Float32x2: UInt32 = 29
public let WGPUVertexFormat_Float32x3: UInt32 = 30
public let WGPUVertexFormat_Float32x4: UInt32 = 31
public let WGPUVertexFormat_Uint32: UInt32 = 32
public let WGPUVertexFormat_Sint32: UInt32 = 36

public let WGPUPrimitiveTopology_PointList: UInt32 = 1
public let WGPUPrimitiveTopology_LineList: UInt32 = 2
public let WGPUPrimitiveTopology_LineStrip: UInt32 = 3
public let WGPUPrimitiveTopology_TriangleList: UInt32 = 4
public let WGPUPrimitiveTopology_TriangleStrip: UInt32 = 5

public let WGPUIndexFormat_Undefined: UInt32 = 0
public let WGPUIndexFormat_Uint16: UInt32 = 1
public let WGPUIndexFormat_Uint32: UInt32 = 2

public let WGPUFrontFace_CCW: UInt32 = 1
public let WGPUFrontFace_CW: UInt32 = 2

public let WGPUCullMode_None: UInt32 = 1
public let WGPUCullMode_Front: UInt32 = 2
public let WGPUCullMode_Back: UInt32 = 3

public let WGPUCompareFunction_Undefined: UInt32 = 0
public let WGPUCompareFunction_Never: UInt32 = 1
public let WGPUCompareFunction_Less: UInt32 = 2
public let WGPUCompareFunction_Equal: UInt32 = 3
public let WGPUCompareFunction_LessEqual: UInt32 = 4
public let WGPUCompareFunction_Greater: UInt32 = 5
public let WGPUCompareFunction_NotEqual: UInt32 = 6
public let WGPUCompareFunction_GreaterEqual: UInt32 = 7
public let WGPUCompareFunction_Always: UInt32 = 8

public let WGPUStencilOperation_Keep: UInt32 = 1

public let WGPULoadOp_Undefined: UInt32 = 0
public let WGPULoadOp_Load: UInt32 = 1
public let WGPULoadOp_Clear: UInt32 = 2
public let WGPUStoreOp_Undefined: UInt32 = 0
public let WGPUStoreOp_Store: UInt32 = 1
public let WGPUStoreOp_Discard: UInt32 = 2

public let WGPUBlendOperation_Add: UInt32 = 1
public let WGPUBlendFactor_Zero: UInt32 = 1
public let WGPUBlendFactor_One: UInt32 = 2
public let WGPUBlendFactor_SrcAlpha: UInt32 = 5
public let WGPUBlendFactor_OneMinusSrcAlpha: UInt32 = 6

public let WGPUColorWriteMask_All: UInt64 = 15

public let WGPUAddressMode_ClampToEdge: UInt32 = 1
public let WGPUAddressMode_Repeat: UInt32 = 2
public let WGPUAddressMode_MirrorRepeat: UInt32 = 3
public let WGPUFilterMode_Nearest: UInt32 = 1
public let WGPUFilterMode_Linear: UInt32 = 2
public let WGPUMipmapFilterMode_Nearest: UInt32 = 1
public let WGPUMipmapFilterMode_Linear: UInt32 = 2

public let WGPUOptionalBool_False: UInt32 = 0
public let WGPUOptionalBool_True: UInt32 = 1

public let WGPUCompositeAlphaMode_Auto: UInt32 = 0

public let WGPUPresentMode_Fifo: UInt32 = 1
public let WGPUPresentMode_FifoRelaxed: UInt32 = 2
public let WGPUPresentMode_Immediate: UInt32 = 3
public let WGPUPresentMode_Mailbox: UInt32 = 4

public let WGPUSurfaceGetCurrentTextureStatus_SuccessOptimal: UInt32 = 1
public let WGPUSurfaceGetCurrentTextureStatus_SuccessSuboptimal: UInt32 = 2
public let WGPUSurfaceGetCurrentTextureStatus_Timeout: UInt32 = 3
public let WGPUSurfaceGetCurrentTextureStatus_Outdated: UInt32 = 4
public let WGPUSurfaceGetCurrentTextureStatus_Lost: UInt32 = 5

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
public type WGPURequestAdapterCallback = @extern(.C) (UInt32, RawPointer, WGPUStringView, RawPointer, RawPointer) -> ()
/// `void (*)(WGPURequestDeviceStatus, WGPUDevice, WGPUStringView, void*, void*)`
public type WGPURequestDeviceCallback = @extern(.C) (UInt32, RawPointer, WGPUStringView, RawPointer, RawPointer) -> ()
/// `void (*)(WGPUMapAsyncStatus, WGPUStringView, void*, void*)`
public type WGPUBufferMapCallback = @extern(.C) (UInt32, WGPUStringView, RawPointer, RawPointer) -> ()
/// `void (*)(WGPUPopErrorScopeStatus, WGPUErrorType, WGPUStringView, void*, void*)`
public type WGPUPopErrorScopeCallback = @extern(.C) (UInt32, UInt32, WGPUStringView, RawPointer, RawPointer) -> ()
/// `void (*)(WGPUDevice const*, WGPUErrorType, WGPUStringView, void*, void*)`
public type WGPUUncapturedErrorCallback = @extern(.C) (RawPointer, UInt32, WGPUStringView, RawPointer, RawPointer) -> ()
/// `void (*)(WGPUDevice const*, WGPUDeviceLostReason, WGPUStringView, void*, void*)`
public type WGPUDeviceLostCallback = @extern(.C) (RawPointer, UInt32, WGPUStringView, RawPointer, RawPointer) -> ()

public struct WGPURequestAdapterCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var mode: UInt32
    public var callback: NullableCFunction[WGPURequestAdapterCallback]
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPURequestDeviceCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var mode: UInt32
    public var callback: NullableCFunction[WGPURequestDeviceCallback]
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPUBufferMapCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var mode: UInt32
    public var callback: NullableCFunction[WGPUBufferMapCallback]
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPUPopErrorScopeCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var mode: UInt32
    public var callback: NullableCFunction[WGPUPopErrorScopeCallback]
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPUUncapturedErrorCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var callback: NullableCFunction[WGPUUncapturedErrorCallback]
    public var userdata1: RawPointer
    public var userdata2: RawPointer
}

public struct WGPUDeviceLostCallbackInfo: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var mode: UInt32
    public var callback: NullableCFunction[WGPUDeviceLostCallback]
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

// ----- Textures and samplers -----

public struct WGPUExtent3D: FFISafe {
    public var width: UInt32
    public var height: UInt32
    public var depthOrArrayLayers: UInt32
}

public struct WGPUOrigin3D: FFISafe {
    public var x: UInt32
    public var y: UInt32
    public var z: UInt32
}

public struct WGPUTextureDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
    public var usage: UInt64
    public var dimension: UInt32
    public var size: WGPUExtent3D
    public var format: UInt32
    public var mipLevelCount: UInt32
    public var sampleCount: UInt32
    public var viewFormatCount: UInt64
    public var viewFormats: Pointer[UInt32]
}

public struct WGPUTextureViewDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
    public var format: UInt32
    public var dimension: UInt32
    public var baseMipLevel: UInt32
    public var mipLevelCount: UInt32
    public var baseArrayLayer: UInt32
    public var arrayLayerCount: UInt32
    public var aspect: UInt32
    public var usage: UInt64
}

public struct WGPUSamplerDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
    public var addressModeU: UInt32
    public var addressModeV: UInt32
    public var addressModeW: UInt32
    public var magFilter: UInt32
    public var minFilter: UInt32
    public var mipmapFilter: UInt32
    public var lodMinClamp: Float32
    public var lodMaxClamp: Float32
    public var compare: UInt32
    public var maxAnisotropy: UInt16
}

public struct WGPUTexelCopyTextureInfo: FFISafe {
    public var texture: WGPUTexture
    public var mipLevel: UInt32
    public var origin: WGPUOrigin3D
    public var aspect: UInt32
}

public struct WGPUTexelCopyBufferLayout: FFISafe {
    public var offset: UInt64
    public var bytesPerRow: UInt32
    public var rowsPerImage: UInt32
}

public struct WGPUTexelCopyBufferInfo: FFISafe {
    public var layout: WGPUTexelCopyBufferLayout
    public var buffer: WGPUBuffer
}

// ----- Render pipelines -----

public struct WGPUVertexAttribute: FFISafe {
    public var format: UInt32
    public var offset: UInt64
    public var shaderLocation: UInt32
}

public struct WGPUVertexBufferLayout: FFISafe {
    public var stepMode: UInt32
    public var arrayStride: UInt64
    public var attributeCount: UInt64
    public var attributes: Pointer[WGPUVertexAttribute]
}

public struct WGPUVertexState: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var shaderModule: WGPUShaderModule  // `module` in C
    public var entryPoint: WGPUStringView
    public var constantCount: UInt64
    public var constants: RawPointer
    public var bufferCount: UInt64
    public var buffers: Pointer[WGPUVertexBufferLayout]
}

public struct WGPUPrimitiveState: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var topology: UInt32
    public var stripIndexFormat: UInt32
    public var frontFace: UInt32
    public var cullMode: UInt32
    public var unclippedDepth: UInt32
}

public struct WGPUStencilFaceState: FFISafe {
    public var compare: UInt32
    public var failOp: UInt32
    public var depthFailOp: UInt32
    public var passOp: UInt32
}

public struct WGPUDepthStencilState: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var format: UInt32
    public var depthWriteEnabled: UInt32
    public var depthCompare: UInt32
    public var stencilFront: WGPUStencilFaceState
    public var stencilBack: WGPUStencilFaceState
    public var stencilReadMask: UInt32
    public var stencilWriteMask: UInt32
    public var depthBias: Int32
    public var depthBiasSlopeScale: Float32
    public var depthBiasClamp: Float32
}

public struct WGPUMultisampleState: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var count: UInt32
    public var mask: UInt32
    public var alphaToCoverageEnabled: UInt32
}

public struct WGPUBlendComponent: FFISafe {
    public var operation: UInt32
    public var srcFactor: UInt32
    public var dstFactor: UInt32
}

public struct WGPUBlendState: FFISafe {
    public var color: WGPUBlendComponent
    public var alpha: WGPUBlendComponent
}

public struct WGPUColorTargetState: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var format: UInt32
    public var blend: Pointer[WGPUBlendState]
    public var writeMask: UInt64
}

public struct WGPUFragmentState: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var shaderModule: WGPUShaderModule  // `module` in C
    public var entryPoint: WGPUStringView
    public var constantCount: UInt64
    public var constants: RawPointer
    public var targetCount: UInt64
    public var targets: Pointer[WGPUColorTargetState]
}

public struct WGPURenderPipelineDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
    public var layout: RawPointer
    public var vertex: WGPUVertexState
    public var primitive: WGPUPrimitiveState
    public var depthStencil: Pointer[WGPUDepthStencilState]
    public var multisample: WGPUMultisampleState
    public var fragment: Pointer[WGPUFragmentState]
}

// ----- Render passes -----

public struct WGPUColor: FFISafe {
    public var r: Float64
    public var g: Float64
    public var b: Float64
    public var a: Float64
}

public struct WGPURenderPassColorAttachment: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var view: WGPUTextureView
    public var depthSlice: UInt32
    public var resolveTarget: WGPUTextureView
    public var loadOp: UInt32
    public var storeOp: UInt32
    public var clearValue: WGPUColor
}

public struct WGPURenderPassDepthStencilAttachment: FFISafe {
    public var view: WGPUTextureView
    public var depthLoadOp: UInt32
    public var depthStoreOp: UInt32
    public var depthClearValue: Float32
    public var depthReadOnly: UInt32
    public var stencilLoadOp: UInt32
    public var stencilStoreOp: UInt32
    public var stencilClearValue: UInt32
    public var stencilReadOnly: UInt32
}

public struct WGPURenderPassDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
    public var colorAttachmentCount: UInt64
    public var colorAttachments: Pointer[WGPURenderPassColorAttachment]
    public var depthStencilAttachment: Pointer[WGPURenderPassDepthStencilAttachment]
    public var occlusionQuerySet: RawPointer
    public var timestampWrites: RawPointer
}

// ----- Surfaces -----

public struct WGPUSurfaceDescriptor: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var label: WGPUStringView
}

public struct WGPUSurfaceSourceXlibWindow: FFISafe {
    public var chain: WGPUChainedStruct
    public var display: RawPointer
    public var window: UInt64
}

public struct WGPUSurfaceSourceWaylandSurface: FFISafe {
    public var chain: WGPUChainedStruct
    public var display: RawPointer
    public var surface: RawPointer
}

public struct WGPUSurfaceSourceMetalLayer: FFISafe {
    public var chain: WGPUChainedStruct
    public var layer: RawPointer
}

public struct WGPUSurfaceSourceWindowsHWND: FFISafe {
    public var chain: WGPUChainedStruct
    public var hinstance: RawPointer
    public var hwnd: RawPointer
}

public struct WGPUSurfaceConfiguration: FFISafe {
    public var nextInChain: Pointer[WGPUChainedStruct]
    public var device: WGPUDevice
    public var format: UInt32
    public var usage: UInt64
    public var width: UInt32
    public var height: UInt32
    public var viewFormatCount: UInt64
    public var viewFormats: Pointer[UInt32]
    public var alphaMode: UInt32
    public var presentMode: UInt32
}

public struct WGPUSurfaceCapabilities: FFISafe {
    public var nextInChain: RawPointer
    public var usages: UInt64
    public var formatCount: UInt64
    public var formats: Pointer[UInt32]
    public var presentModeCount: UInt64
    public var presentModes: Pointer[UInt32]
    public var alphaModeCount: UInt64
    public var alphaModes: Pointer[UInt32]
}

public struct WGPUSurfaceTexture: FFISafe {
    public var nextInChain: RawPointer
    public var texture: WGPUTexture
    public var status: UInt32
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

// ----- Textures and samplers -----

@extern(.C, mangleName: "wgpuDeviceCreateTexture")
public func wgpuDeviceCreateTexture(device: WGPUDevice, descriptor: &WGPUTextureDescriptor) -> WGPUTexture
@extern(.C, mangleName: "wgpuTextureCreateView")
public func wgpuTextureCreateView(texture: WGPUTexture, descriptor: RawPointer) -> WGPUTextureView
@extern(.C, mangleName: "wgpuTextureGetWidth")
public func wgpuTextureGetWidth(texture: WGPUTexture) -> UInt32
@extern(.C, mangleName: "wgpuTextureGetHeight")
public func wgpuTextureGetHeight(texture: WGPUTexture) -> UInt32
@extern(.C, mangleName: "wgpuTextureGetFormat")
public func wgpuTextureGetFormat(texture: WGPUTexture) -> UInt32
@extern(.C, mangleName: "wgpuTextureRelease")
public func wgpuTextureRelease(texture: WGPUTexture)
@extern(.C, mangleName: "wgpuTextureViewRelease")
public func wgpuTextureViewRelease(textureView: WGPUTextureView)
@extern(.C, mangleName: "wgpuDeviceCreateSampler")
public func wgpuDeviceCreateSampler(device: WGPUDevice, descriptor: &WGPUSamplerDescriptor) -> WGPUSampler
@extern(.C, mangleName: "wgpuSamplerRelease")
public func wgpuSamplerRelease(sampler: WGPUSampler)
@extern(.C, mangleName: "wgpuQueueWriteTexture")
public func wgpuQueueWriteTexture(queue: WGPUQueue, destination: &WGPUTexelCopyTextureInfo, data: RawPointer, dataSize: UInt64, dataLayout: &WGPUTexelCopyBufferLayout, writeSize: &WGPUExtent3D)
@extern(.C, mangleName: "wgpuCommandEncoderCopyTextureToBuffer")
public func wgpuCommandEncoderCopyTextureToBuffer(commandEncoder: WGPUCommandEncoder, source: &WGPUTexelCopyTextureInfo, destination: &WGPUTexelCopyBufferInfo, copySize: &WGPUExtent3D)

// ----- Render pipelines and passes -----

@extern(.C, mangleName: "wgpuDeviceCreateRenderPipeline")
public func wgpuDeviceCreateRenderPipeline(device: WGPUDevice, descriptor: &WGPURenderPipelineDescriptor) -> WGPURenderPipeline
@extern(.C, mangleName: "wgpuRenderPipelineGetBindGroupLayout")
public func wgpuRenderPipelineGetBindGroupLayout(renderPipeline: WGPURenderPipeline, groupIndex: UInt32) -> WGPUBindGroupLayout
@extern(.C, mangleName: "wgpuRenderPipelineRelease")
public func wgpuRenderPipelineRelease(renderPipeline: WGPURenderPipeline)

@extern(.C, mangleName: "wgpuCommandEncoderBeginRenderPass")
public func wgpuCommandEncoderBeginRenderPass(commandEncoder: WGPUCommandEncoder, descriptor: &WGPURenderPassDescriptor) -> WGPURenderPassEncoder
@extern(.C, mangleName: "wgpuRenderPassEncoderSetPipeline")
public func wgpuRenderPassEncoderSetPipeline(renderPassEncoder: WGPURenderPassEncoder, pipeline: WGPURenderPipeline)
@extern(.C, mangleName: "wgpuRenderPassEncoderSetBindGroup")
public func wgpuRenderPassEncoderSetBindGroup(renderPassEncoder: WGPURenderPassEncoder, groupIndex: UInt32, group: WGPUBindGroup, dynamicOffsetCount: UInt64, dynamicOffsets: RawPointer)
@extern(.C, mangleName: "wgpuRenderPassEncoderSetVertexBuffer")
public func wgpuRenderPassEncoderSetVertexBuffer(renderPassEncoder: WGPURenderPassEncoder, slot: UInt32, buffer: WGPUBuffer, offset: UInt64, size: UInt64)
@extern(.C, mangleName: "wgpuRenderPassEncoderSetIndexBuffer")
public func wgpuRenderPassEncoderSetIndexBuffer(renderPassEncoder: WGPURenderPassEncoder, buffer: WGPUBuffer, format: UInt32, offset: UInt64, size: UInt64)
@extern(.C, mangleName: "wgpuRenderPassEncoderSetViewport")
public func wgpuRenderPassEncoderSetViewport(renderPassEncoder: WGPURenderPassEncoder, x: Float32, y: Float32, width: Float32, height: Float32, minDepth: Float32, maxDepth: Float32)
@extern(.C, mangleName: "wgpuRenderPassEncoderSetScissorRect")
public func wgpuRenderPassEncoderSetScissorRect(renderPassEncoder: WGPURenderPassEncoder, x: UInt32, y: UInt32, width: UInt32, height: UInt32)
@extern(.C, mangleName: "wgpuRenderPassEncoderDraw")
public func wgpuRenderPassEncoderDraw(renderPassEncoder: WGPURenderPassEncoder, vertexCount: UInt32, instanceCount: UInt32, firstVertex: UInt32, firstInstance: UInt32)
@extern(.C, mangleName: "wgpuRenderPassEncoderDrawIndexed")
public func wgpuRenderPassEncoderDrawIndexed(renderPassEncoder: WGPURenderPassEncoder, indexCount: UInt32, instanceCount: UInt32, firstIndex: UInt32, baseVertex: Int32, firstInstance: UInt32)
@extern(.C, mangleName: "wgpuRenderPassEncoderEnd")
public func wgpuRenderPassEncoderEnd(renderPassEncoder: WGPURenderPassEncoder)
@extern(.C, mangleName: "wgpuRenderPassEncoderRelease")
public func wgpuRenderPassEncoderRelease(renderPassEncoder: WGPURenderPassEncoder)

// ----- Surfaces -----

@extern(.C, mangleName: "wgpuInstanceCreateSurface")
public func wgpuInstanceCreateSurface(instance: WGPUInstance, descriptor: &WGPUSurfaceDescriptor) -> WGPUSurface
@extern(.C, mangleName: "wgpuSurfaceGetCapabilities")
public func wgpuSurfaceGetCapabilities(surface: WGPUSurface, adapter: WGPUAdapter, capabilities: &mutating WGPUSurfaceCapabilities) -> UInt32
@extern(.C, mangleName: "wgpuSurfaceCapabilitiesFreeMembers")
public func wgpuSurfaceCapabilitiesFreeMembers(surfaceCapabilities: WGPUSurfaceCapabilities)
@extern(.C, mangleName: "wgpuSurfaceConfigure")
public func wgpuSurfaceConfigure(surface: WGPUSurface, config: &WGPUSurfaceConfiguration)
@extern(.C, mangleName: "wgpuSurfaceGetCurrentTexture")
public func wgpuSurfaceGetCurrentTexture(surface: WGPUSurface, surfaceTexture: &mutating WGPUSurfaceTexture)
@extern(.C, mangleName: "wgpuSurfacePresent")
public func wgpuSurfacePresent(surface: WGPUSurface) -> UInt32
@extern(.C, mangleName: "wgpuSurfaceUnconfigure")
public func wgpuSurfaceUnconfigure(surface: WGPUSurface)
@extern(.C, mangleName: "wgpuSurfaceRelease")
public func wgpuSurfaceRelease(surface: WGPUSurface)
