// Explicit bind group and pipeline layouts, push constant ranges, and the
// device features and limits that unlock them.

module wgpu

import wgpu.raw

// ===== Shader stages =====

/// A shader stage that can see a binding or push constants.
public enum ShaderStage {
    case Vertex
    case Fragment
    case Compute

    var bits: UInt64 {
        match self {
            .Vertex => raw.WGPUShaderStage_Vertex,
            .Fragment => raw.WGPUShaderStage_Fragment,
            .Compute => raw.WGPUShaderStage_Compute
        }
    }
}

/// `stages` as the C bit set.
func stageBits(stages: [ShaderStage]) -> UInt64 {
    var bits: UInt64 = 0;
    for s in stages {
        bits = bits | s.bits;
    }
    bits
}

// ===== Bind group layouts =====

/// What a shader reads from a texture binding.
public enum TextureSampleType {
    /// Floats, filterable: `texture_2d<f32>`.
    case Float
    /// Floats a filtering sampler can't read, such as `R32Float`.
    case UnfilterableFloat
    /// A depth texture: `texture_depth_2d`.
    case Depth
    case Sint
    case Uint

    var cValue: UInt32 {
        match self {
            .Float => raw.WGPUTextureSampleType_Float,
            .UnfilterableFloat => raw.WGPUTextureSampleType_UnfilterableFloat,
            .Depth => raw.WGPUTextureSampleType_Depth,
            .Sint => raw.WGPUTextureSampleType_Sint,
            .Uint => raw.WGPUTextureSampleType_Uint
        }
    }
}

/// The kind of a sampler binding.
public enum SamplerBindingType {
    case Filtering
    case NonFiltering
    /// A `sampler_comparison`, for shadow maps.
    case Comparison

    var cValue: UInt32 {
        match self {
            .Filtering => raw.WGPUSamplerBindingType_Filtering,
            .NonFiltering => raw.WGPUSamplerBindingType_NonFiltering,
            .Comparison => raw.WGPUSamplerBindingType_Comparison
        }
    }
}

/// How a shader may touch a storage texture.
public enum StorageTextureAccess {
    case WriteOnly
    case ReadOnly
    case ReadWrite

    var cValue: UInt32 {
        match self {
            .WriteOnly => raw.WGPUStorageTextureAccess_WriteOnly,
            .ReadOnly => raw.WGPUStorageTextureAccess_ReadOnly,
            .ReadWrite => raw.WGPUStorageTextureAccess_ReadWrite
        }
    }
}

/// One binding of a bind group layout: its `@binding` index, the stages
/// that see it, and what kind of resource it is. Make one with
/// `uniformBuffer`, `storageBuffer`, `texture`, `storageTexture` or
/// `sampler`.
public struct BindingLayout {
    var entry: raw.WGPUBindGroupLayoutEntry

    /// An entry using none of the resource kinds; each constructor fills one.
    static func empty(binding: Int64, visibility: [ShaderStage]) -> raw.WGPUBindGroupLayoutEntry {
        raw.WGPUBindGroupLayoutEntry(
            nextInChain: Pointer.nullPointer(),
            binding: UInt32(from: binding),
            visibility: stageBits(visibility),
            buffer: raw.WGPUBufferBindingLayout(nextInChain: Pointer.nullPointer(), kind: raw.WGPUBufferBindingType_BindingNotUsed, hasDynamicOffset: 0, minBindingSize: 0),
            sampler: raw.WGPUSamplerBindingLayout(nextInChain: Pointer.nullPointer(), kind: raw.WGPUSamplerBindingType_BindingNotUsed),
            texture: raw.WGPUTextureBindingLayout(nextInChain: Pointer.nullPointer(), sampleType: raw.WGPUTextureSampleType_BindingNotUsed, viewDimension: raw.WGPUTextureViewDimension_Undefined, multisampled: 0),
            storageTexture: raw.WGPUStorageTextureBindingLayout(nextInChain: Pointer.nullPointer(), access: raw.WGPUStorageTextureAccess_BindingNotUsed, format: raw.WGPUTextureFormat_Undefined, viewDimension: raw.WGPUTextureViewDimension_Undefined)
        )
    }

    /// A `var<uniform>` buffer. A `dynamic` one takes its byte offset at
    /// `setBindGroup`, so one buffer can hold many draws' uniforms.
    public static func uniformBuffer(binding: Int64, visibility visibility: [ShaderStage], dynamic dynamic: Bool = false, minSize minSize: Int64 = 0) -> BindingLayout {
        var e = BindingLayout.empty(binding, visibility);
        e.buffer = raw.WGPUBufferBindingLayout(
            nextInChain: Pointer.nullPointer(),
            kind: raw.WGPUBufferBindingType_Uniform,
            hasDynamicOffset: if dynamic { 1 } else { 0 },
            minBindingSize: UInt64(from: minSize)
        );
        BindingLayout(entry: e)
    }

    /// A `var<storage>` buffer, `read_write` unless `readOnly`.
    public static func storageBuffer(binding: Int64, visibility visibility: [ShaderStage], readOnly readOnly: Bool = false, dynamic dynamic: Bool = false, minSize minSize: Int64 = 0) -> BindingLayout {
        var e = BindingLayout.empty(binding, visibility);
        e.buffer = raw.WGPUBufferBindingLayout(
            nextInChain: Pointer.nullPointer(),
            kind: if readOnly { raw.WGPUBufferBindingType_ReadOnlyStorage } else { raw.WGPUBufferBindingType_Storage },
            hasDynamicOffset: if dynamic { 1 } else { 0 },
            minBindingSize: UInt64(from: minSize)
        );
        BindingLayout(entry: e)
    }

    /// A sampled texture.
    public static func texture(binding: Int64, visibility visibility: [ShaderStage], sampleType sampleType: TextureSampleType = .Float, dimension dimension: TextureViewDimension = .D2, multisampled multisampled: Bool = false) -> BindingLayout {
        var e = BindingLayout.empty(binding, visibility);
        e.texture = raw.WGPUTextureBindingLayout(
            nextInChain: Pointer.nullPointer(),
            sampleType: sampleType.cValue,
            viewDimension: dimension.cValue,
            multisampled: if multisampled { 1 } else { 0 }
        );
        BindingLayout(entry: e)
    }

    /// A storage texture of `format`, which shaders load and store texels in.
    public static func storageTexture(binding: Int64, visibility visibility: [ShaderStage], format format: TextureFormat, access access: StorageTextureAccess = .WriteOnly, dimension dimension: TextureViewDimension = .D2) -> BindingLayout {
        var e = BindingLayout.empty(binding, visibility);
        e.storageTexture = raw.WGPUStorageTextureBindingLayout(
            nextInChain: Pointer.nullPointer(),
            access: access.cValue,
            format: format.cValue,
            viewDimension: dimension.cValue
        );
        BindingLayout(entry: e)
    }

    /// A sampler.
    public static func sampler(binding: Int64, visibility visibility: [ShaderStage], kind kind: SamplerBindingType = .Filtering) -> BindingLayout {
        var e = BindingLayout.empty(binding, visibility);
        e.sampler = raw.WGPUSamplerBindingLayout(nextInChain: Pointer.nullPointer(), kind: kind.cValue);
        BindingLayout(entry: e)
    }
}

extend BindGroupLayout {
    /// Another handle to the same layout, to hand to a second owner.
    public func share() -> BindGroupLayout {
        unsafe raw.wgpuBindGroupLayoutAddRef(self.handle);
        BindGroupLayout(handle: self.handle)
    }
}

// ===== Pipeline layouts =====

/// Bytes `start` to `end` of the push constants, seen by `stages`.
public struct PushConstantRange {
    /// The first `size` bytes, seen by `stages`.
    public static func first(size: Int64, stages stages: [ShaderStage]) -> PushConstantRange {
        PushConstantRange(stages: stages, start: 0, end: size)
    }

    public var stages: [ShaderStage]
    public var start: Int64
    public var end: Int64

    public init(stages stages: [ShaderStage], start start: Int64, end end: Int64) {
        self.stages = stages;
        self.start = start;
        self.end = end;
    }
}

/// The bind group layouts (and push constants) a pipeline uses. Pipelines
/// made with the same layout share bind groups.
public struct PipelineLayout: not Copyable {
    var handle: raw.WGPUPipelineLayout
    /// The bind group layouts, owned by this value.
    var groups: [raw.WGPUBindGroupLayout]

    /// The layout of bind group `group`, for `Device.createBindGroup`.
    public func bindGroupLayout(group: Int64) -> BindGroupLayout {
        let h = self.groups(group);
        unsafe raw.wgpuBindGroupLayoutAddRef(h);
        BindGroupLayout(handle: h)
    }

    /// Another handle to the same layout, to give to a second pipeline.
    public func share() -> PipelineLayout {
        unsafe raw.wgpuPipelineLayoutAddRef(self.handle);
        for g in self.groups {
            unsafe raw.wgpuBindGroupLayoutAddRef(g);
        }
        PipelineLayout(handle: self.handle, groups: self.groups)
    }

    deinit {
        for g in self.groups {
            unsafe raw.wgpuBindGroupLayoutRelease(g);
        }
        unsafe raw.wgpuPipelineLayoutRelease(self.handle);
    }
}

extend Device {
    /// A bind group layout with `entries`.
    public func createBindGroupLayout(entries entries: [BindingLayout], label label: String = "") -> BindGroupLayout {
        BindGroupLayout(handle: self.bindGroupLayoutHandle(entries, label))
    }

    func bindGroupLayoutHandle(entries: [BindingLayout], label: String) -> raw.WGPUBindGroupLayout {
        var list: [raw.WGPUBindGroupLayoutEntry] = [];
        for e in entries {
            list.append(e.entry);
        }
        let descriptor = raw.WGPUBindGroupLayoutDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            entryCount: UInt64(from: list.count),
            entries: list.asPointer()
        );
        unsafe raw.wgpuDeviceCreateBindGroupLayout(self.handle, descriptor)
    }

    /// A pipeline layout whose bind group `i` has the entries `groups(i)`,
    /// with `pushConstants` ranges (which need `Feature.PushConstants`).
    public func createPipelineLayout(groups groups: [[BindingLayout]], pushConstants pushConstants: [PushConstantRange] = [], label label: String = "") -> PipelineLayout {
        var handles: [raw.WGPUBindGroupLayout] = [];
        for g in groups {
            handles.append(self.bindGroupLayoutHandle(g, label));
        }
        var ranges: [raw.WGPUPushConstantRange] = [];
        for r in pushConstants {
            ranges.append(raw.WGPUPushConstantRange(stages: stageBits(r.stages), start: UInt32(from: r.start), end: UInt32(from: r.end)));
        }
        var extras = raw.WGPUPipelineLayoutExtras(
            chain: raw.WGPUChainedStruct(next: Pointer.nullPointer(), sType: raw.WGPUSType_PipelineLayoutExtras),
            pushConstantRangeCount: UInt64(from: ranges.count),
            pushConstantRanges: ranges.asPointer()
        );
        let descriptor = raw.WGPUPipelineLayoutDescriptor(
            nextInChain: if ranges.count > 0 { Pointer(to: extras).cast[raw.WGPUChainedStruct]() } else { Pointer.nullPointer() },
            label: view(label),
            bindGroupLayoutCount: UInt64(from: handles.count),
            bindGroupLayouts: handles.asPointer()
        );
        PipelineLayout(handle: unsafe raw.wgpuDeviceCreatePipelineLayout(self.handle, descriptor), groups: handles)
    }
}

// ===== Features and limits =====

/// An optional capability a device can be asked for, if its adapter
/// supports it.
public enum Feature {
    /// Pipelines that don't clip at the near and far planes.
    case DepthClipControl
    /// The `Depth32FloatStencil8` texture format.
    case Depth32FloatStencil8
    /// Timestamp query sets and pass timestamps.
    case TimestampQuery
    /// A nonzero `firstInstance` in indirect draws.
    case IndirectFirstInstance
    /// `f16` in shaders.
    case ShaderF16
    /// Filtering samplers on 32-bit float textures.
    case Float32Filterable
    /// wgpu-native's push constants: small per-draw data set right in the
    /// pass. Ask for it with `requestDevice(maxPushConstantSize:)`.
    case PushConstants

    var cValue: UInt32 {
        match self {
            .DepthClipControl => raw.WGPUFeatureName_DepthClipControl,
            .Depth32FloatStencil8 => raw.WGPUFeatureName_Depth32FloatStencil8,
            .TimestampQuery => raw.WGPUFeatureName_TimestampQuery,
            .IndirectFirstInstance => raw.WGPUFeatureName_IndirectFirstInstance,
            .ShaderF16 => raw.WGPUFeatureName_ShaderF16,
            .Float32Filterable => raw.WGPUFeatureName_Float32Filterable,
            .PushConstants => raw.WGPUNativeFeature_PushConstants
        }
    }
}

/// How far a device can go: the limits most programs check.
public struct Limits {
    public var maxTextureDimension2D: Int64
    public var maxTextureArrayLayers: Int64
    public var maxBindGroups: Int64
    public var maxVertexBuffers: Int64
    public var maxVertexAttributes: Int64
    public var maxColorAttachments: Int64
    public var maxBufferSize: Int64
    public var maxUniformBufferBindingSize: Int64
    public var maxStorageBufferBindingSize: Int64
    /// What a dynamic uniform buffer's offsets must be a multiple of.
    public var minUniformBufferOffsetAlignment: Int64
    /// What a dynamic storage buffer's offsets must be a multiple of.
    public var minStorageBufferOffsetAlignment: Int64
    public var maxComputeWorkgroupsPerDimension: Int64
    public var maxComputeInvocationsPerWorkgroup: Int64

    static func fromC(l: raw.WGPULimits) -> Limits {
        Limits(
            maxTextureDimension2D: Int64(from: l.maxTextureDimension2D),
            maxTextureArrayLayers: Int64(from: l.maxTextureArrayLayers),
            maxBindGroups: Int64(from: l.maxBindGroups),
            maxVertexBuffers: Int64(from: l.maxVertexBuffers),
            maxVertexAttributes: Int64(from: l.maxVertexAttributes),
            maxColorAttachments: Int64(from: l.maxColorAttachments),
            maxBufferSize: Int64(from: l.maxBufferSize),
            maxUniformBufferBindingSize: Int64(from: l.maxUniformBufferBindingSize),
            maxStorageBufferBindingSize: Int64(from: l.maxStorageBufferBindingSize),
            minUniformBufferOffsetAlignment: Int64(from: l.minUniformBufferOffsetAlignment),
            minStorageBufferOffsetAlignment: Int64(from: l.minStorageBufferOffsetAlignment),
            maxComputeWorkgroupsPerDimension: Int64(from: l.maxComputeWorkgroupsPerDimension),
            maxComputeInvocationsPerWorkgroup: Int64(from: l.maxComputeInvocationsPerWorkgroup)
        )
    }
}

/// A `WGPULimits` with every limit undefined, meaning "the default", for
/// `getLimits` to fill or `requestDevice` to raise one of.
func undefinedLimits(next: Pointer[raw.WGPUChainedStruct]) -> raw.WGPULimits {
    let u = raw.WGPU_U32_UNDEFINED;
    let u64 = raw.WGPU_U64_UNDEFINED;
    raw.WGPULimits(
        nextInChain: next,
        maxTextureDimension1D: u, maxTextureDimension2D: u, maxTextureDimension3D: u,
        maxTextureArrayLayers: u, maxBindGroups: u, maxBindGroupsPlusVertexBuffers: u,
        maxBindingsPerBindGroup: u, maxDynamicUniformBuffersPerPipelineLayout: u,
        maxDynamicStorageBuffersPerPipelineLayout: u, maxSampledTexturesPerShaderStage: u,
        maxSamplersPerShaderStage: u, maxStorageBuffersPerShaderStage: u,
        maxStorageTexturesPerShaderStage: u, maxUniformBuffersPerShaderStage: u,
        maxUniformBufferBindingSize: u64, maxStorageBufferBindingSize: u64,
        minUniformBufferOffsetAlignment: u, minStorageBufferOffsetAlignment: u,
        maxVertexBuffers: u, maxBufferSize: u64, maxVertexAttributes: u,
        maxVertexBufferArrayStride: u, maxInterStageShaderVariables: u,
        maxColorAttachments: u, maxColorAttachmentBytesPerSample: u,
        maxComputeWorkgroupStorageSize: u, maxComputeInvocationsPerWorkgroup: u,
        maxComputeWorkgroupSizeX: u, maxComputeWorkgroupSizeY: u, maxComputeWorkgroupSizeZ: u,
        maxComputeWorkgroupsPerDimension: u
    )
}

extend Adapter {
    /// Whether devices from this adapter can have `feature`.
    public func supports(feature: Feature) -> Bool {
        unsafe raw.wgpuAdapterHasFeature(self.handle, feature.cValue) != 0
    }

    /// The best limits a device from this adapter can have.
    public var limits: Limits {
        var l = undefinedLimits(Pointer.nullPointer());
        unsafe raw.wgpuAdapterGetLimits(self.handle, l);
        Limits.fromC(l)
    }
}

extend Device {
    /// Whether the device was created with `feature`.
    public func supports(feature: Feature) -> Bool {
        unsafe raw.wgpuDeviceHasFeature(self.handle, feature.cValue) != 0
    }

    /// The device's limits.
    public var limits: Limits {
        var l = undefinedLimits(Pointer.nullPointer());
        unsafe raw.wgpuDeviceGetLimits(self.handle, l);
        Limits.fromC(l)
    }
}
