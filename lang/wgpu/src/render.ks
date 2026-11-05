// Textures, samplers, render pipelines and render passes.

module wgpu

import wgpu.raw

// ===== Textures =====

/// How a texture's texels are stored.
public enum TextureFormat {
    case R8Unorm
    case R32Float
    case RGBA8Unorm
    case RGBA8UnormSrgb
    case BGRA8Unorm
    case BGRA8UnormSrgb
    case RGBA16Float
    case RGBA32Float
    case Depth16Unorm
    case Depth24Plus
    case Depth24PlusStencil8
    case Depth32Float
    /// A format this wrapper has no name for, by its `WGPUTextureFormat`.
    case Other(UInt32)

    var cValue: UInt32 {
        match self {
            .R8Unorm => raw.WGPUTextureFormat_R8Unorm,
            .R32Float => raw.WGPUTextureFormat_R32Float,
            .RGBA8Unorm => raw.WGPUTextureFormat_RGBA8Unorm,
            .RGBA8UnormSrgb => raw.WGPUTextureFormat_RGBA8UnormSrgb,
            .BGRA8Unorm => raw.WGPUTextureFormat_BGRA8Unorm,
            .BGRA8UnormSrgb => raw.WGPUTextureFormat_BGRA8UnormSrgb,
            .RGBA16Float => raw.WGPUTextureFormat_RGBA16Float,
            .RGBA32Float => raw.WGPUTextureFormat_RGBA32Float,
            .Depth16Unorm => raw.WGPUTextureFormat_Depth16Unorm,
            .Depth24Plus => raw.WGPUTextureFormat_Depth24Plus,
            .Depth24PlusStencil8 => raw.WGPUTextureFormat_Depth24PlusStencil8,
            .Depth32Float => raw.WGPUTextureFormat_Depth32Float,
            .Other(f) => f
        }
    }

    static func fromC(f: UInt32) -> TextureFormat {
        if f == raw.WGPUTextureFormat_R8Unorm { return .R8Unorm }
        if f == raw.WGPUTextureFormat_R32Float { return .R32Float }
        if f == raw.WGPUTextureFormat_RGBA8Unorm { return .RGBA8Unorm }
        if f == raw.WGPUTextureFormat_RGBA8UnormSrgb { return .RGBA8UnormSrgb }
        if f == raw.WGPUTextureFormat_BGRA8Unorm { return .BGRA8Unorm }
        if f == raw.WGPUTextureFormat_BGRA8UnormSrgb { return .BGRA8UnormSrgb }
        if f == raw.WGPUTextureFormat_RGBA16Float { return .RGBA16Float }
        if f == raw.WGPUTextureFormat_RGBA32Float { return .RGBA32Float }
        if f == raw.WGPUTextureFormat_Depth16Unorm { return .Depth16Unorm }
        if f == raw.WGPUTextureFormat_Depth24Plus { return .Depth24Plus }
        if f == raw.WGPUTextureFormat_Depth24PlusStencil8 { return .Depth24PlusStencil8 }
        if f == raw.WGPUTextureFormat_Depth32Float { return .Depth32Float }
        .Other(f)
    }

    /// Bytes per texel of a color format; 0 for depth formats, which can't
    /// be copied to a buffer whole.
    public var bytesPerTexel: Int64 {
        match self {
            .R8Unorm => 1,
            .R32Float => 4,
            .RGBA8Unorm => 4,
            .RGBA8UnormSrgb => 4,
            .BGRA8Unorm => 4,
            .BGRA8UnormSrgb => 4,
            .RGBA16Float => 8,
            .RGBA32Float => 16,
            _ => 0
        }
    }
}

/// What a texture may be used for.
public enum TextureUsage {
    case CopySrc
    case CopyDst
    case TextureBinding
    case StorageBinding
    case RenderAttachment

    var bits: UInt64 {
        match self {
            .CopySrc => raw.WGPUTextureUsage_CopySrc,
            .CopyDst => raw.WGPUTextureUsage_CopyDst,
            .TextureBinding => raw.WGPUTextureUsage_TextureBinding,
            .StorageBinding => raw.WGPUTextureUsage_StorageBinding,
            .RenderAttachment => raw.WGPUTextureUsage_RenderAttachment
        }
    }
}

/// An image on the GPU.
public struct Texture: not Copyable {
    var handle: raw.WGPUTexture

    /// The width in texels.
    public var width: Int64 { Int64(from: unsafe raw.wgpuTextureGetWidth(self.handle)) }

    /// The height in texels.
    public var height: Int64 { Int64(from: unsafe raw.wgpuTextureGetHeight(self.handle)) }

    /// The texel format.
    public var format: TextureFormat { TextureFormat.fromC(unsafe raw.wgpuTextureGetFormat(self.handle)) }

    /// The bytes per row a buffer copy of this texture uses: the texel row,
    /// rounded up to the 256 bytes WebGPU requires.
    public var paddedBytesPerRow: Int64 {
        let row = self.width * self.format.bytesPerTexel;
        (row + 255) / 256 * 256
    }

    /// A view of the whole texture, to render to or bind.
    public func createView() -> TextureView {
        TextureView(handle: unsafe raw.wgpuTextureCreateView(self.handle, RawPointer.nullPointer()))
    }

    deinit {
        unsafe raw.wgpuTextureRelease(self.handle);
    }
}

/// A texture as a render target or shader resource.
public struct TextureView: not Copyable {
    var handle: raw.WGPUTextureView

    /// This view at `@binding(index)`, for `Device.createBindGroup`.
    public func binding(index: Int64) -> Binding {
        Binding(entry: raw.WGPUBindGroupEntry(
            nextInChain: Pointer.nullPointer(),
            binding: UInt32(from: index),
            buffer: RawPointer.nullPointer(),
            offset: 0,
            size: 0,
            sampler: RawPointer.nullPointer(),
            textureView: self.handle
        ))
    }

    /// Render into this view, first clearing it to `clear`, or keeping what
    /// it holds when `clear` is `None`.
    public func colorAttachment(clear clear: Color? = .Some(Color.black)) -> ColorAttachment {
        ColorAttachment(cValue: raw.WGPURenderPassColorAttachment(
            nextInChain: Pointer.nullPointer(),
            view: self.handle,
            depthSlice: raw.WGPU_U32_UNDEFINED,
            resolveTarget: RawPointer.nullPointer(),
            loadOp: if clear.isSome() { raw.WGPULoadOp_Clear } else { raw.WGPULoadOp_Load },
            storeOp: raw.WGPUStoreOp_Store,
            clearValue: (clear ?? Color.black).cValue
        ))
    }

    /// Use this view (of a depth texture) as the depth buffer, first clearing
    /// it to `clear`.
    public func depthAttachment(clear clear: Float32 = 1.0) -> DepthAttachment {
        DepthAttachment(cValue: raw.WGPURenderPassDepthStencilAttachment(
            view: self.handle,
            depthLoadOp: raw.WGPULoadOp_Clear,
            depthStoreOp: raw.WGPUStoreOp_Store,
            depthClearValue: clear,
            depthReadOnly: 0,
            stencilLoadOp: raw.WGPULoadOp_Undefined,
            stencilStoreOp: raw.WGPUStoreOp_Undefined,
            stencilClearValue: 0,
            stencilReadOnly: 0
        ))
    }

    deinit {
        unsafe raw.wgpuTextureViewRelease(self.handle);
    }
}

/// How a sampler picks between texels.
public enum FilterMode {
    case Nearest
    case Linear
}

/// What a sampler does outside 0 to 1.
public enum AddressMode {
    case ClampToEdge
    case Repeat
    case MirrorRepeat

    var cValue: UInt32 {
        match self {
            .ClampToEdge => raw.WGPUAddressMode_ClampToEdge,
            .Repeat => raw.WGPUAddressMode_Repeat,
            .MirrorRepeat => raw.WGPUAddressMode_MirrorRepeat
        }
    }
}

/// How shaders read a texture.
public struct Sampler: not Copyable {
    var handle: raw.WGPUSampler

    /// This sampler at `@binding(index)`, for `Device.createBindGroup`.
    public func binding(index: Int64) -> Binding {
        Binding(entry: raw.WGPUBindGroupEntry(
            nextInChain: Pointer.nullPointer(),
            binding: UInt32(from: index),
            buffer: RawPointer.nullPointer(),
            offset: 0,
            size: 0,
            sampler: self.handle,
            textureView: RawPointer.nullPointer()
        ))
    }

    deinit {
        unsafe raw.wgpuSamplerRelease(self.handle);
    }
}

// ===== Pipelines =====

/// The type of one vertex attribute.
public enum VertexFormat {
    case Float32
    case Float32x2
    case Float32x3
    case Float32x4
    case Uint32
    case Sint32
    /// Four bytes read as 0 to 1, such as an RGBA8 color.
    case Unorm8x4

    var cValue: UInt32 {
        match self {
            .Float32 => raw.WGPUVertexFormat_Float32,
            .Float32x2 => raw.WGPUVertexFormat_Float32x2,
            .Float32x3 => raw.WGPUVertexFormat_Float32x3,
            .Float32x4 => raw.WGPUVertexFormat_Float32x4,
            .Uint32 => raw.WGPUVertexFormat_Uint32,
            .Sint32 => raw.WGPUVertexFormat_Sint32,
            .Unorm8x4 => raw.WGPUVertexFormat_Unorm8x4
        }
    }

    /// The size in bytes.
    public var size: Int64 {
        match self {
            .Float32 => 4,
            .Float32x2 => 8,
            .Float32x3 => 12,
            .Float32x4 => 16,
            .Uint32 => 4,
            .Sint32 => 4,
            .Unorm8x4 => 4
        }
    }
}

/// One attribute of a vertex: `@location(location)` at byte `offset`.
public struct VertexAttribute {
    public var format: VertexFormat
    public var offset: Int64
    public var location: Int64
}

/// Whether a vertex buffer advances per vertex or per instance.
public enum VertexStepMode {
    case Vertex
    case Instance
}

/// How one vertex buffer's elements map to shader inputs.
public struct VertexBufferLayout {
    /// Bytes from one element to the next.
    public var stride: Int64
    public var attributes: [VertexAttribute]
    public var stepMode: VertexStepMode

    public init(stride stride: Int64, attributes attributes: [VertexAttribute], stepMode stepMode: VertexStepMode = .Vertex) {
        self.stride = stride;
        self.attributes = attributes;
        self.stepMode = stepMode;
    }

    /// @name Packed
    /// Attributes `formats` back to back, at `@location(0)`, `@location(1)`
    /// and so on, as a struct of those fields lays them out.
    public init(packed formats: [VertexFormat], stepMode stepMode: VertexStepMode = .Vertex) {
        var attributes: [VertexAttribute] = [];
        var offset: Int64 = 0;
        for f in formats {
            attributes.append(VertexAttribute(format: f, offset: offset, location: attributes.count));
            offset = offset + f.size;
        }
        self.stride = offset;
        self.attributes = attributes;
        self.stepMode = stepMode;
    }
}

/// How vertices form primitives.
public enum PrimitiveTopology {
    case TriangleList
    case TriangleStrip
    case LineList
    case LineStrip
    case PointList

    var cValue: UInt32 {
        match self {
            .TriangleList => raw.WGPUPrimitiveTopology_TriangleList,
            .TriangleStrip => raw.WGPUPrimitiveTopology_TriangleStrip,
            .LineList => raw.WGPUPrimitiveTopology_LineList,
            .LineStrip => raw.WGPUPrimitiveTopology_LineStrip,
            .PointList => raw.WGPUPrimitiveTopology_PointList
        }
    }
}

/// Which triangles to skip drawing.
public enum CullMode {
    case None
    case Front
    case Back
}

/// Which winding is a triangle's front.
public enum FrontFace {
    case CounterClockwise
    case Clockwise
}

/// A depth or stencil test.
public enum CompareFunction {
    case Never
    case Less
    case Equal
    case LessEqual
    case Greater
    case NotEqual
    case GreaterEqual
    case Always

    var cValue: UInt32 {
        match self {
            .Never => raw.WGPUCompareFunction_Never,
            .Less => raw.WGPUCompareFunction_Less,
            .Equal => raw.WGPUCompareFunction_Equal,
            .LessEqual => raw.WGPUCompareFunction_LessEqual,
            .Greater => raw.WGPUCompareFunction_Greater,
            .NotEqual => raw.WGPUCompareFunction_NotEqual,
            .GreaterEqual => raw.WGPUCompareFunction_GreaterEqual,
            .Always => raw.WGPUCompareFunction_Always
        }
    }
}

/// How a fragment's color combines with the target's.
public enum BlendMode {
    /// Overwrite the target.
    case Replace
    /// Standard transparency: `src * srcAlpha + dst * (1 - srcAlpha)`.
    case AlphaBlending
}

/// One color output of a render pipeline.
public struct ColorTarget {
    public var format: TextureFormat
    public var blend: BlendMode

    public init(format format: TextureFormat, blend blend: BlendMode = .Replace) {
        self.format = format;
        self.blend = blend;
    }
}

/// A render pipeline's depth test.
public struct DepthTest {
    public var format: TextureFormat
    public var compare: CompareFunction
    /// Whether passing fragments write their depth.
    public var write: Bool

    public init(format format: TextureFormat = .Depth24Plus, compare compare: CompareFunction = .Less, write write: Bool = true) {
        self.format = format;
        self.compare = compare;
        self.write = write;
    }
}

/// A vertex and fragment shader pair with its fixed-function state, ready
/// to draw.
public struct RenderPipeline: not Copyable {
    var handle: raw.WGPURenderPipeline

    /// The layout of bind group `group`, inferred from the shaders.
    public func bindGroupLayout(group: Int64) -> BindGroupLayout {
        BindGroupLayout(handle: unsafe raw.wgpuRenderPipelineGetBindGroupLayout(self.handle, UInt32(from: group)))
    }

    deinit {
        unsafe raw.wgpuRenderPipelineRelease(self.handle);
    }
}

extend Device {
    /// Create a texture of `width` by `height` texels.
    public func createTexture(width width: Int64, height height: Int64, format format: TextureFormat, usage usage: [TextureUsage], label label: String = "") -> Texture {
        var bits: UInt64 = 0;
        for u in usage {
            bits = bits | u.bits;
        }
        let descriptor = raw.WGPUTextureDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            usage: bits,
            dimension: raw.WGPUTextureDimension_2D,
            size: raw.WGPUExtent3D(width: UInt32(from: width), height: UInt32(from: height), depthOrArrayLayers: 1),
            format: format.cValue,
            mipLevelCount: 1,
            sampleCount: 1,
            viewFormatCount: 0,
            viewFormats: Pointer.nullPointer()
        );
        Texture(handle: unsafe raw.wgpuDeviceCreateTexture(self.handle, descriptor))
    }

    /// Copy tightly packed texel rows `data` into the whole of `texture`.
    public func write[T](texture texture: Texture, data data: [T]) where T: FFISafe {
        let destination = raw.WGPUTexelCopyTextureInfo(
            texture: texture.handle,
            mipLevel: 0,
            origin: raw.WGPUOrigin3D(x: 0, y: 0, z: 0),
            aspect: raw.WGPUTextureAspect_All
        );
        let width = texture.width;
        let height = texture.height;
        let layout = raw.WGPUTexelCopyBufferLayout(
            offset: 0,
            bytesPerRow: UInt32(from: width * texture.format.bytesPerTexel),
            rowsPerImage: UInt32(from: height)
        );
        let size = raw.WGPUExtent3D(width: UInt32(from: width), height: UInt32(from: height), depthOrArrayLayers: 1);
        let bytes = Layout.array[T](data.count).size;
        unsafe raw.wgpuQueueWriteTexture(self.queue, destination, data.asPointer().asRaw(), UInt64(from: bytes), layout, size);
    }

    /// Create a sampler.
    public func createSampler(filter filter: FilterMode = .Linear, addressMode addressMode: AddressMode = .ClampToEdge, label label: String = "") -> Sampler {
        let f = match filter {
            .Nearest => raw.WGPUFilterMode_Nearest,
            .Linear => raw.WGPUFilterMode_Linear
        };
        let mip = match filter {
            .Nearest => raw.WGPUMipmapFilterMode_Nearest,
            .Linear => raw.WGPUMipmapFilterMode_Linear
        };
        let descriptor = raw.WGPUSamplerDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            addressModeU: addressMode.cValue,
            addressModeV: addressMode.cValue,
            addressModeW: addressMode.cValue,
            magFilter: f,
            minFilter: f,
            mipmapFilter: mip,
            lodMinClamp: 0.0,
            lodMaxClamp: 32.0,
            compare: raw.WGPUCompareFunction_Undefined,
            maxAnisotropy: 1
        );
        Sampler(handle: unsafe raw.wgpuDeviceCreateSampler(self.handle, descriptor))
    }

    /// Create a render pipeline from `shader`'s `vertexEntry` and
    /// `fragmentEntry` functions, reading `buffers` and writing `targets`.
    /// Bind group layouts are inferred from the shader.
    public func createRenderPipeline(
        shader shader: ShaderModule,
        targets targets: [ColorTarget],
        buffers buffers: [VertexBufferLayout] = [],
        vertexEntry vertexEntry: String = "vs_main",
        fragmentEntry fragmentEntry: String = "fs_main",
        topology topology: PrimitiveTopology = .TriangleList,
        cullMode cullMode: CullMode = .None,
        frontFace frontFace: FrontFace = .CounterClockwise,
        depth depth: DepthTest? = .None,
        label label: String = ""
    ) -> RenderPipeline {
        // Every array the descriptor points into lives until the call.
        var attributes: [[raw.WGPUVertexAttribute]] = [];
        for b in buffers {
            var list: [raw.WGPUVertexAttribute] = [];
            for a in b.attributes {
                list.append(raw.WGPUVertexAttribute(format: a.format.cValue, offset: UInt64(from: a.offset), shaderLocation: UInt32(from: a.location)));
            }
            attributes.append(list);
        }
        var layouts: [raw.WGPUVertexBufferLayout] = [];
        var i = 0;
        while i < buffers.count {
            let b = buffers(i);
            layouts.append(raw.WGPUVertexBufferLayout(
                stepMode: match b.stepMode {
                    .Vertex => raw.WGPUVertexStepMode_Vertex,
                    .Instance => raw.WGPUVertexStepMode_Instance
                },
                arrayStride: UInt64(from: b.stride),
                attributeCount: UInt64(from: attributes(i).count),
                attributes: attributes(i).asPointer()
            ));
            i = i + 1;
        }
        var blend = raw.WGPUBlendState(
            color: raw.WGPUBlendComponent(operation: raw.WGPUBlendOperation_Add, srcFactor: raw.WGPUBlendFactor_SrcAlpha, dstFactor: raw.WGPUBlendFactor_OneMinusSrcAlpha),
            alpha: raw.WGPUBlendComponent(operation: raw.WGPUBlendOperation_Add, srcFactor: raw.WGPUBlendFactor_One, dstFactor: raw.WGPUBlendFactor_OneMinusSrcAlpha)
        );
        var colorTargets: [raw.WGPUColorTargetState] = [];
        for t in targets {
            colorTargets.append(raw.WGPUColorTargetState(
                nextInChain: Pointer.nullPointer(),
                format: t.format.cValue,
                blend: match t.blend {
                    .Replace => Pointer.nullPointer(),
                    .AlphaBlending => Pointer(to: blend)
                },
                writeMask: raw.WGPUColorWriteMask_All
            ));
        }
        var fragment = raw.WGPUFragmentState(
            nextInChain: Pointer.nullPointer(),
            shaderModule: shader.handle,
            entryPoint: view(fragmentEntry),
            constantCount: 0,
            constants: RawPointer.nullPointer(),
            targetCount: UInt64(from: colorTargets.count),
            targets: colorTargets.asPointer()
        );
        let keep = raw.WGPUStencilFaceState(
            compare: raw.WGPUCompareFunction_Always,
            failOp: raw.WGPUStencilOperation_Keep,
            depthFailOp: raw.WGPUStencilOperation_Keep,
            passOp: raw.WGPUStencilOperation_Keep
        );
        let test = depth ?? DepthTest();
        var depthState = raw.WGPUDepthStencilState(
            nextInChain: Pointer.nullPointer(),
            format: test.format.cValue,
            depthWriteEnabled: if test.write { raw.WGPUOptionalBool_True } else { raw.WGPUOptionalBool_False },
            depthCompare: test.compare.cValue,
            stencilFront: keep,
            stencilBack: keep,
            stencilReadMask: 0,
            stencilWriteMask: 0,
            depthBias: 0,
            depthBiasSlopeScale: 0.0,
            depthBiasClamp: 0.0
        );
        let descriptor = raw.WGPURenderPipelineDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            layout: RawPointer.nullPointer(),
            vertex: raw.WGPUVertexState(
                nextInChain: Pointer.nullPointer(),
                shaderModule: shader.handle,
                entryPoint: view(vertexEntry),
                constantCount: 0,
                constants: RawPointer.nullPointer(),
                bufferCount: UInt64(from: layouts.count),
                buffers: layouts.asPointer()
            ),
            primitive: raw.WGPUPrimitiveState(
                nextInChain: Pointer.nullPointer(),
                topology: topology.cValue,
                stripIndexFormat: raw.WGPUIndexFormat_Undefined,
                frontFace: match frontFace {
                    .CounterClockwise => raw.WGPUFrontFace_CCW,
                    .Clockwise => raw.WGPUFrontFace_CW
                },
                cullMode: match cullMode {
                    .None => raw.WGPUCullMode_None,
                    .Front => raw.WGPUCullMode_Front,
                    .Back => raw.WGPUCullMode_Back
                },
                unclippedDepth: 0
            ),
            depthStencil: if depth.isSome() { Pointer(to: depthState) } else { Pointer.nullPointer() },
            multisample: raw.WGPUMultisampleState(nextInChain: Pointer.nullPointer(), count: 1, mask: 4294967295, alphaToCoverageEnabled: 0),
            fragment: Pointer(to: fragment)
        );
        RenderPipeline(handle: unsafe raw.wgpuDeviceCreateRenderPipeline(self.handle, descriptor))
    }

    /// Read `texture` (made with `.CopySrc`) back as tightly packed rows of
    /// bytes, waiting for the GPU. Color formats only.
    public func read(texture texture: Texture) -> [UInt8] throws WgpuError {
        let padded = texture.paddedBytesPerRow;
        let height = texture.height;
        let row = texture.width * texture.format.bytesPerTexel;
        let buffer = self.createBuffer(size: padded * height, usage: [.MapRead, .CopyDst], label: "texture readback");
        let encoder = self.createCommandEncoder();
        encoder.copy(texture: texture, to: buffer);
        self.submit(encoder.finish());
        let bytes: [UInt8] = try self.read(buffer);
        var out: [UInt8] = [];
        var y = 0;
        while y < height {
            var x = 0;
            while x < row {
                out.append(bytes(y * padded + x));
                x = x + 1;
            }
            y = y + 1;
        }
        .Ok(out)
    }
}

// ===== Render passes =====

/// A color in linear RGBA, each 0 to 1.
public struct Color {
    public var r: Float64
    public var g: Float64
    public var b: Float64
    public var a: Float64

    public init(r r: Float64, g g: Float64, b b: Float64, a a: Float64 = 1.0) {
        self.r = r;
        self.g = g;
        self.b = b;
        self.a = a;
    }

    public static var black: Color { Color(r: 0.0, g: 0.0, b: 0.0) }
    public static var white: Color { Color(r: 1.0, g: 1.0, b: 1.0) }
    public static var transparent: Color { Color(r: 0.0, g: 0.0, b: 0.0, a: 0.0) }

    var cValue: raw.WGPUColor { raw.WGPUColor(r: self.r, g: self.g, b: self.b, a: self.a) }
}

/// A view a render pass draws into, from `TextureView.colorAttachment`.
/// The view must outlive the pass.
public struct ColorAttachment {
    var cValue: raw.WGPURenderPassColorAttachment
}

/// A depth buffer for a render pass, from `TextureView.depthAttachment`.
/// The view must outlive the pass.
public struct DepthAttachment {
    var cValue: raw.WGPURenderPassDepthStencilAttachment
}

/// The type of an index buffer's indices.
public enum IndexFormat {
    case Uint16
    case Uint32
}

extend CommandEncoder {
    /// Record a render pass drawing into `color`: `body` sets pipelines and
    /// draws.
    public func renderPass(color color: [ColorAttachment], body: (RenderPass) -> ()) {
        self.beginRenderPass(color, .None, body)
    }

    /// Record a render pass drawing into `color` and testing against
    /// `depth`.
    public func renderPass(color color: [ColorAttachment], depth depth: DepthAttachment, body: (RenderPass) -> ()) {
        self.beginRenderPass(color, .Some(depth), body)
    }

    func beginRenderPass(color: [ColorAttachment], depth: DepthAttachment?, body: (RenderPass) -> ()) {
        var attachments: [raw.WGPURenderPassColorAttachment] = [];
        for c in color {
            attachments.append(c.cValue);
        }
        var depthRaw = match depth {
            .Some(d) => d.cValue,
            .None => raw.WGPURenderPassDepthStencilAttachment(
                view: RawPointer.nullPointer(),
                depthLoadOp: 0, depthStoreOp: 0, depthClearValue: 0.0, depthReadOnly: 0,
                stencilLoadOp: 0, stencilStoreOp: 0, stencilClearValue: 0, stencilReadOnly: 0
            )
        };
        let descriptor = raw.WGPURenderPassDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: noString(),
            colorAttachmentCount: UInt64(from: attachments.count),
            colorAttachments: attachments.asPointer(),
            depthStencilAttachment: if depth.isSome() { Pointer(to: depthRaw) } else { Pointer.nullPointer() },
            occlusionQuerySet: RawPointer.nullPointer(),
            timestampWrites: RawPointer.nullPointer()
        );
        let pass = RenderPass(handle: unsafe raw.wgpuCommandEncoderBeginRenderPass(self.handle, descriptor));
        body(pass);
        unsafe raw.wgpuRenderPassEncoderEnd(pass.handle);
    }

    /// Copy all of `texture` (a color format, made with `.CopySrc`) into
    /// `buffer`, rows `texture.paddedBytesPerRow` apart.
    public func copy(texture texture: Texture, to buffer: Buffer) {
        let source = raw.WGPUTexelCopyTextureInfo(
            texture: texture.handle,
            mipLevel: 0,
            origin: raw.WGPUOrigin3D(x: 0, y: 0, z: 0),
            aspect: raw.WGPUTextureAspect_All
        );
        let destination = raw.WGPUTexelCopyBufferInfo(
            layout: raw.WGPUTexelCopyBufferLayout(
                offset: 0,
                bytesPerRow: UInt32(from: texture.paddedBytesPerRow),
                rowsPerImage: UInt32(from: texture.height)
            ),
            buffer: buffer.handle
        );
        let size = raw.WGPUExtent3D(width: UInt32(from: texture.width), height: UInt32(from: texture.height), depthOrArrayLayers: 1);
        unsafe raw.wgpuCommandEncoderCopyTextureToBuffer(self.handle, source, destination, size);
    }
}

/// A render pass being recorded.
public struct RenderPass: not Copyable {
    var handle: raw.WGPURenderPassEncoder

    /// Use `pipeline` for the next draws.
    public func setPipeline(pipeline: RenderPipeline) {
        unsafe raw.wgpuRenderPassEncoderSetPipeline(self.handle, pipeline.handle);
    }

    /// Bind `bindGroup` at `index`.
    public func setBindGroup(index: Int64, bindGroup: BindGroup) {
        unsafe raw.wgpuRenderPassEncoderSetBindGroup(self.handle, UInt32(from: index), bindGroup.handle, 0, RawPointer.nullPointer());
    }

    /// Read vertex buffer `slot` from `buffer`.
    public func setVertexBuffer(slot: Int64, buffer: Buffer) {
        unsafe raw.wgpuRenderPassEncoderSetVertexBuffer(self.handle, UInt32(from: slot), buffer.handle, 0, UInt64(from: buffer.size));
    }

    /// Read indices from `buffer`.
    public func setIndexBuffer(buffer: Buffer, format format: IndexFormat = .Uint32) {
        let f = match format {
            .Uint16 => raw.WGPUIndexFormat_Uint16,
            .Uint32 => raw.WGPUIndexFormat_Uint32
        };
        unsafe raw.wgpuRenderPassEncoderSetIndexBuffer(self.handle, buffer.handle, f, 0, UInt64(from: buffer.size));
    }

    /// Draw only into the given rectangle, in pixels.
    public func setViewport(x: Float32, y: Float32, width: Float32, height: Float32) {
        unsafe raw.wgpuRenderPassEncoderSetViewport(self.handle, x, y, width, height, 0.0, 1.0);
    }

    /// Discard fragments outside the given rectangle, in pixels.
    public func setScissor(x: Int64, y: Int64, width: Int64, height: Int64) {
        unsafe raw.wgpuRenderPassEncoderSetScissorRect(self.handle, UInt32(from: x), UInt32(from: y), UInt32(from: width), UInt32(from: height));
    }

    /// Draw `vertices` vertices, `instances` times.
    public func draw(vertices vertices: Int64, instances instances: Int64 = 1, firstVertex firstVertex: Int64 = 0) {
        unsafe raw.wgpuRenderPassEncoderDraw(self.handle, UInt32(from: vertices), UInt32(from: instances), UInt32(from: firstVertex), 0);
    }

    /// Draw `indices` indices from the index buffer, `instances` times.
    public func drawIndexed(indices indices: Int64, instances instances: Int64 = 1, firstIndex firstIndex: Int64 = 0) {
        unsafe raw.wgpuRenderPassEncoderDrawIndexed(self.handle, UInt32(from: indices), UInt32(from: instances), UInt32(from: firstIndex), 0, 0);
    }

    deinit {
        unsafe raw.wgpuRenderPassEncoderRelease(self.handle);
    }
}
