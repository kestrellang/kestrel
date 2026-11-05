// Textures, samplers, render pipelines and render passes.

module wgpu

import wgpu.raw

// ===== Textures =====

/// How a texture's texels are stored.
public enum TextureFormat {
    case R8Unorm
    case R8Snorm
    case R8Uint
    case R16Float
    case R32Float
    case R32Uint
    case R32Sint
    case RG8Unorm
    case RG16Float
    case RG32Float
    case RGBA8Unorm
    case RGBA8UnormSrgb
    case RGBA8Snorm
    case RGBA8Uint
    case BGRA8Unorm
    case BGRA8UnormSrgb
    case RGB10A2Unorm
    case RG11B10Ufloat
    case RGBA16Float
    case RGBA16Uint
    case RGBA32Float
    case RGBA32Uint
    case Stencil8
    case Depth16Unorm
    case Depth24Plus
    case Depth24PlusStencil8
    case Depth32Float
    /// Needs `Feature.Depth32FloatStencil8`.
    case Depth32FloatStencil8
    /// A format this wrapper has no name for, by its `WGPUTextureFormat`.
    case Other(UInt32)

    /// Every named format, for looking one up by its C value.
    static var named: [TextureFormat] {
        [
            .R8Unorm, .R8Snorm, .R8Uint, .R16Float, .R32Float, .R32Uint, .R32Sint,
            .RG8Unorm, .RG16Float, .RG32Float,
            .RGBA8Unorm, .RGBA8UnormSrgb, .RGBA8Snorm, .RGBA8Uint, .BGRA8Unorm, .BGRA8UnormSrgb,
            .RGB10A2Unorm, .RG11B10Ufloat, .RGBA16Float, .RGBA16Uint, .RGBA32Float, .RGBA32Uint,
            .Stencil8, .Depth16Unorm, .Depth24Plus, .Depth24PlusStencil8, .Depth32Float, .Depth32FloatStencil8
        ]
    }

    var cValue: UInt32 {
        match self {
            .R8Unorm => raw.WGPUTextureFormat_R8Unorm,
            .R8Snorm => raw.WGPUTextureFormat_R8Snorm,
            .R8Uint => raw.WGPUTextureFormat_R8Uint,
            .R16Float => raw.WGPUTextureFormat_R16Float,
            .R32Float => raw.WGPUTextureFormat_R32Float,
            .R32Uint => raw.WGPUTextureFormat_R32Uint,
            .R32Sint => raw.WGPUTextureFormat_R32Sint,
            .RG8Unorm => raw.WGPUTextureFormat_RG8Unorm,
            .RG16Float => raw.WGPUTextureFormat_RG16Float,
            .RG32Float => raw.WGPUTextureFormat_RG32Float,
            .RGBA8Unorm => raw.WGPUTextureFormat_RGBA8Unorm,
            .RGBA8UnormSrgb => raw.WGPUTextureFormat_RGBA8UnormSrgb,
            .RGBA8Snorm => raw.WGPUTextureFormat_RGBA8Snorm,
            .RGBA8Uint => raw.WGPUTextureFormat_RGBA8Uint,
            .BGRA8Unorm => raw.WGPUTextureFormat_BGRA8Unorm,
            .BGRA8UnormSrgb => raw.WGPUTextureFormat_BGRA8UnormSrgb,
            .RGB10A2Unorm => raw.WGPUTextureFormat_RGB10A2Unorm,
            .RG11B10Ufloat => raw.WGPUTextureFormat_RG11B10Ufloat,
            .RGBA16Float => raw.WGPUTextureFormat_RGBA16Float,
            .RGBA16Uint => raw.WGPUTextureFormat_RGBA16Uint,
            .RGBA32Float => raw.WGPUTextureFormat_RGBA32Float,
            .RGBA32Uint => raw.WGPUTextureFormat_RGBA32Uint,
            .Stencil8 => raw.WGPUTextureFormat_Stencil8,
            .Depth16Unorm => raw.WGPUTextureFormat_Depth16Unorm,
            .Depth24Plus => raw.WGPUTextureFormat_Depth24Plus,
            .Depth24PlusStencil8 => raw.WGPUTextureFormat_Depth24PlusStencil8,
            .Depth32Float => raw.WGPUTextureFormat_Depth32Float,
            .Depth32FloatStencil8 => raw.WGPUTextureFormat_Depth32FloatStencil8,
            .Other(f) => f
        }
    }

    static func fromC(f: UInt32) -> TextureFormat {
        for format in TextureFormat.named {
            if format.cValue == f {
                return format;
            }
        }
        .Other(f)
    }

    /// Bytes per texel of a color format; 0 for depth and stencil formats,
    /// which can't be copied whole.
    public var bytesPerTexel: Int64 {
        match self {
            .R8Unorm => 1,
            .R8Snorm => 1,
            .R8Uint => 1,
            .R16Float => 2,
            .R32Float => 4,
            .R32Uint => 4,
            .R32Sint => 4,
            .RG8Unorm => 2,
            .RG16Float => 4,
            .RG32Float => 8,
            .RGBA8Unorm => 4,
            .RGBA8UnormSrgb => 4,
            .RGBA8Snorm => 4,
            .RGBA8Uint => 4,
            .BGRA8Unorm => 4,
            .BGRA8UnormSrgb => 4,
            .RGB10A2Unorm => 4,
            .RG11B10Ufloat => 4,
            .RGBA16Float => 8,
            .RGBA16Uint => 8,
            .RGBA32Float => 16,
            .RGBA32Uint => 16,
            _ => 0
        }
    }

    /// Whether the format has a depth aspect.
    public var hasDepth: Bool {
        match self {
            .Depth16Unorm => true,
            .Depth24Plus => true,
            .Depth24PlusStencil8 => true,
            .Depth32Float => true,
            .Depth32FloatStencil8 => true,
            _ => false
        }
    }

    /// Whether the format has a stencil aspect.
    public var hasStencil: Bool {
        match self {
            .Stencil8 => true,
            .Depth24PlusStencil8 => true,
            .Depth32FloatStencil8 => true,
            _ => false
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

/// The shape of a texture.
public enum TextureDimension {
    /// A row of texels.
    case D1
    /// A 2D image, or an array of them (a cube map is six layers).
    case D2
    /// A volume of texels.
    case D3

    var cValue: UInt32 {
        match self {
            .D1 => raw.WGPUTextureDimension_1D,
            .D2 => raw.WGPUTextureDimension_2D,
            .D3 => raw.WGPUTextureDimension_3D
        }
    }
}

/// How a view presents a texture to shaders.
public enum TextureViewDimension {
    case D1
    case D2
    case D2Array
    /// Six layers of a 2D texture as a cube.
    case Cube
    case CubeArray
    case D3

    var cValue: UInt32 {
        match self {
            .D1 => raw.WGPUTextureViewDimension_1D,
            .D2 => raw.WGPUTextureViewDimension_2D,
            .D2Array => raw.WGPUTextureViewDimension_2DArray,
            .Cube => raw.WGPUTextureViewDimension_Cube,
            .CubeArray => raw.WGPUTextureViewDimension_CubeArray,
            .D3 => raw.WGPUTextureViewDimension_3D
        }
    }
}

/// Which parts of a depth-stencil texture a view or copy covers.
public enum TextureAspect {
    case All
    case DepthOnly
    case StencilOnly

    var cValue: UInt32 {
        match self {
            .All => raw.WGPUTextureAspect_All,
            .DepthOnly => raw.WGPUTextureAspect_DepthOnly,
            .StencilOnly => raw.WGPUTextureAspect_StencilOnly
        }
    }
}

/// Which mip levels and layers of a texture a view covers.
public struct ViewRange {
    /// How the view presents the texture; `None` infers it from the texture.
    public var dimension: TextureViewDimension?
    public var baseMipLevel: Int64
    /// How many mip levels; `None` for all from `baseMipLevel` on.
    public var mipLevels: Int64?
    public var baseLayer: Int64
    /// How many array layers; `None` for all from `baseLayer` on.
    public var layers: Int64?
    public var aspect: TextureAspect

    public init(
        dimension dimension: TextureViewDimension? = .None,
        baseMipLevel baseMipLevel: Int64 = 0,
        mipLevels mipLevels: Int64? = .None,
        baseLayer baseLayer: Int64 = 0,
        layers layers: Int64? = .None,
        aspect aspect: TextureAspect = .All
    ) {
        self.dimension = dimension;
        self.baseMipLevel = baseMipLevel;
        self.mipLevels = mipLevels;
        self.baseLayer = baseLayer;
        self.layers = layers;
        self.aspect = aspect;
    }

    /// Just mip level `level` of layer `layer`, as a 2D image: what a render
    /// pass draws into.
    public static func level(level: Int64, layer layer: Int64 = 0) -> ViewRange {
        ViewRange(dimension: .Some(.D2), baseMipLevel: level, mipLevels: .Some(1), baseLayer: layer, layers: .Some(1))
    }

    /// The six layers of a 2D texture from `baseLayer`, as a cube map.
    public static func cube(baseLayer baseLayer: Int64 = 0) -> ViewRange {
        ViewRange(dimension: .Some(.Cube), baseLayer: baseLayer, layers: .Some(6))
    }
}

/// An image on the GPU.
public struct Texture: not Copyable {
    var handle: raw.WGPUTexture

    /// The width in texels.
    public var width: Int64 { Int64(from: unsafe raw.wgpuTextureGetWidth(self.handle)) }

    /// The height in texels.
    public var height: Int64 { Int64(from: unsafe raw.wgpuTextureGetHeight(self.handle)) }

    /// Array layers of a 1D or 2D texture, or the depth of a 3D one.
    public var layers: Int64 { Int64(from: unsafe raw.wgpuTextureGetDepthOrArrayLayers(self.handle)) }

    /// How many mip levels it has.
    public var mipLevelCount: Int64 { Int64(from: unsafe raw.wgpuTextureGetMipLevelCount(self.handle)) }

    /// Samples per texel: 1, or 4 for a multisampled texture.
    public var sampleCount: Int64 { Int64(from: unsafe raw.wgpuTextureGetSampleCount(self.handle)) }

    /// The texel format.
    public var format: TextureFormat { TextureFormat.fromC(unsafe raw.wgpuTextureGetFormat(self.handle)) }

    public var dimension: TextureDimension {
        let d = unsafe raw.wgpuTextureGetDimension(self.handle);
        if d == raw.WGPUTextureDimension_1D { return .D1 }
        if d == raw.WGPUTextureDimension_3D { return .D3 }
        .D2
    }

    /// The width of mip level `level`.
    public func levelWidth(level: Int64) -> Int64 { mipSize(self.width, level) }

    /// The height of mip level `level`.
    public func levelHeight(level: Int64) -> Int64 { mipSize(self.height, level) }

    /// The bytes per row a buffer copy of mip level 0 uses: the texel row,
    /// rounded up to the 256 bytes WebGPU requires.
    public var paddedBytesPerRow: Int64 { self.paddedRowBytes(0) }

    /// The bytes per row a buffer copy of mip level `level` uses.
    public func paddedRowBytes(level: Int64) -> Int64 {
        let row = self.levelWidth(level) * self.format.bytesPerTexel;
        (row + 255) / 256 * 256
    }

    /// How many mip levels a full chain down to 1x1 has for a `width` by
    /// `height` texture.
    public static func fullMipChain(width width: Int64, height height: Int64) -> Int64 {
        var size = if width > height { width } else { height };
        var levels: Int64 = 1;
        while size > 1 {
            size = size / 2;
            levels = levels + 1;
        }
        levels
    }

    /// A view of the whole texture, to render to or bind.
    public func createView() -> TextureView {
        TextureView(handle: unsafe raw.wgpuTextureCreateView(self.handle, RawPointer.nullPointer()), format: self.format)
    }

    /// A view of part of the texture: some mip levels or layers, or the
    /// texture seen as a cube or an array.
    public func createView(range range: ViewRange, label label: String = "") -> TextureView {
        let descriptor = raw.WGPUTextureViewDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            format: raw.WGPUTextureFormat_Undefined,
            dimension: match range.dimension {
                .Some(d) => d.cValue,
                .None => raw.WGPUTextureViewDimension_Undefined
            },
            baseMipLevel: UInt32(from: range.baseMipLevel),
            mipLevelCount: match range.mipLevels {
                .Some(n) => UInt32(from: n),
                .None => raw.WGPU_U32_UNDEFINED
            },
            baseArrayLayer: UInt32(from: range.baseLayer),
            arrayLayerCount: match range.layers {
                .Some(n) => UInt32(from: n),
                .None => raw.WGPU_U32_UNDEFINED
            },
            aspect: range.aspect.cValue,
            usage: 0
        );
        let handle = unsafe raw.wgpuTextureCreateView(self.handle, Pointer(to: descriptor).asRaw());
        TextureView(handle: handle, format: self.format)
    }

    /// Where mip level `level` of layer `layer` sits, for a copy.
    func copyInfo(level: Int64, layer: Int64) -> raw.WGPUTexelCopyTextureInfo {
        raw.WGPUTexelCopyTextureInfo(
            texture: self.handle,
            mipLevel: UInt32(from: level),
            origin: raw.WGPUOrigin3D(x: 0, y: 0, z: UInt32(from: layer)),
            aspect: raw.WGPUTextureAspect_All
        )
    }

    /// The size of one layer of mip level `level`.
    func levelExtent(level: Int64) -> raw.WGPUExtent3D {
        raw.WGPUExtent3D(width: UInt32(from: self.levelWidth(level)), height: UInt32(from: self.levelHeight(level)), depthOrArrayLayers: 1)
    }

    deinit {
        unsafe raw.wgpuTextureRelease(self.handle);
    }
}

/// A side of a texture at mip level `level`: halved per level, at least 1.
func mipSize(size: Int64, level: Int64) -> Int64 {
    var s = size;
    var i: Int64 = 0;
    while i < level and s > 1 {
        s = s / 2;
        i = i + 1;
    }
    s
}

/// A texture as a render target or shader resource.
public struct TextureView: not Copyable {
    var handle: raw.WGPUTextureView
    /// The texture's format, which says whether an attachment needs stencil
    /// operations.
    public let format: TextureFormat

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

    /// Render into this multisampled view and resolve the result into
    /// `target`, a single-sampled view of the same size and format. The
    /// samples themselves are discarded after the pass.
    public func colorAttachment(resolvingInto target: TextureView, clear clear: Color? = .Some(Color.black)) -> ColorAttachment {
        ColorAttachment(cValue: raw.WGPURenderPassColorAttachment(
            nextInChain: Pointer.nullPointer(),
            view: self.handle,
            depthSlice: raw.WGPU_U32_UNDEFINED,
            resolveTarget: target.handle,
            loadOp: if clear.isSome() { raw.WGPULoadOp_Clear } else { raw.WGPULoadOp_Load },
            storeOp: raw.WGPUStoreOp_Discard,
            clearValue: (clear ?? Color.black).cValue
        ))
    }

    /// Use this view (of a depth or stencil texture) as the depth-stencil
    /// buffer. Depth is first cleared to `clear`, or kept when it is `None`;
    /// a stencil aspect is cleared to `stencilClear` along with it. A
    /// `readOnly` buffer is tested against but never written, so it can be
    /// bound to shaders in the same pass.
    public func depthAttachment(clear clear: Float32? = .Some(1.0), stencilClear stencilClear: Int64 = 0, readOnly readOnly: Bool = false) -> DepthAttachment {
        let load = if clear.isSome() { raw.WGPULoadOp_Clear } else { raw.WGPULoadOp_Load };
        let depthOps = self.format.hasDepth and not readOnly;
        let stencilOps = self.format.hasStencil and not readOnly;
        DepthAttachment(cValue: raw.WGPURenderPassDepthStencilAttachment(
            view: self.handle,
            depthLoadOp: if depthOps { load } else { raw.WGPULoadOp_Undefined },
            depthStoreOp: if depthOps { raw.WGPUStoreOp_Store } else { raw.WGPUStoreOp_Undefined },
            depthClearValue: clear ?? 1.0,
            depthReadOnly: if readOnly and self.format.hasDepth { 1 } else { 0 },
            stencilLoadOp: if stencilOps { load } else { raw.WGPULoadOp_Undefined },
            stencilStoreOp: if stencilOps { raw.WGPUStoreOp_Store } else { raw.WGPUStoreOp_Undefined },
            stencilClearValue: UInt32(from: stencilClear),
            stencilReadOnly: if readOnly and self.format.hasStencil { 1 } else { 0 }
        ))
    }

    deinit {
        unsafe raw.wgpuTextureViewRelease(self.handle);
    }
}

// ===== Samplers =====

/// How a sampler picks between texels.
public enum FilterMode {
    case Nearest
    case Linear

    var cValue: UInt32 {
        match self {
            .Nearest => raw.WGPUFilterMode_Nearest,
            .Linear => raw.WGPUFilterMode_Linear
        }
    }

    var mipValue: UInt32 {
        match self {
            .Nearest => raw.WGPUMipmapFilterMode_Nearest,
            .Linear => raw.WGPUMipmapFilterMode_Linear
        }
    }
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
    /// Attributes `formats` back to back, at `@location(firstLocation)` and
    /// on, as a struct of those fields lays them out.
    public init(packed formats: [VertexFormat], stepMode stepMode: VertexStepMode = .Vertex, firstLocation firstLocation: Int64 = 0) {
        var attributes: [VertexAttribute] = [];
        var offset: Int64 = 0;
        for f in formats {
            attributes.append(VertexAttribute(format: f, offset: offset, location: firstLocation + attributes.count));
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

/// A depth, stencil or sampler comparison.
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

/// A term of a blend equation.
public enum BlendFactor {
    case Zero
    case One
    case Src
    case OneMinusSrc
    case SrcAlpha
    case OneMinusSrcAlpha
    case Dst
    case OneMinusDst
    case DstAlpha
    case OneMinusDstAlpha
    case SrcAlphaSaturated
    /// The pass's `setBlendConstant` color.
    case Constant
    case OneMinusConstant

    var cValue: UInt32 {
        match self {
            .Zero => raw.WGPUBlendFactor_Zero,
            .One => raw.WGPUBlendFactor_One,
            .Src => raw.WGPUBlendFactor_Src,
            .OneMinusSrc => raw.WGPUBlendFactor_OneMinusSrc,
            .SrcAlpha => raw.WGPUBlendFactor_SrcAlpha,
            .OneMinusSrcAlpha => raw.WGPUBlendFactor_OneMinusSrcAlpha,
            .Dst => raw.WGPUBlendFactor_Dst,
            .OneMinusDst => raw.WGPUBlendFactor_OneMinusDst,
            .DstAlpha => raw.WGPUBlendFactor_DstAlpha,
            .OneMinusDstAlpha => raw.WGPUBlendFactor_OneMinusDstAlpha,
            .SrcAlphaSaturated => raw.WGPUBlendFactor_SrcAlphaSaturated,
            .Constant => raw.WGPUBlendFactor_Constant,
            .OneMinusConstant => raw.WGPUBlendFactor_OneMinusConstant
        }
    }
}

/// How the two blend terms combine.
public enum BlendOperation {
    case Add
    case Subtract
    case ReverseSubtract
    case Min
    case Max

    var cValue: UInt32 {
        match self {
            .Add => raw.WGPUBlendOperation_Add,
            .Subtract => raw.WGPUBlendOperation_Subtract,
            .ReverseSubtract => raw.WGPUBlendOperation_ReverseSubtract,
            .Min => raw.WGPUBlendOperation_Min,
            .Max => raw.WGPUBlendOperation_Max
        }
    }
}

/// One channel's blend equation: `source * src (operation) target * dst`.
public struct BlendComponent {
    public var source: BlendFactor
    public var destination: BlendFactor
    public var operation: BlendOperation

    public init(source source: BlendFactor, destination destination: BlendFactor, operation operation: BlendOperation = .Add) {
        self.source = source;
        self.destination = destination;
        self.operation = operation;
    }

    var cValue: raw.WGPUBlendComponent {
        raw.WGPUBlendComponent(operation: self.operation.cValue, srcFactor: self.source.cValue, dstFactor: self.destination.cValue)
    }
}

/// How a fragment's color combines with the target's.
public enum BlendMode {
    /// Overwrite the target.
    case Replace
    /// Standard transparency: `src * srcAlpha + dst * (1 - srcAlpha)`.
    case AlphaBlending
    /// Transparency for colors already multiplied by their alpha.
    case PremultipliedAlpha
    /// Add the fragment to the target, as for light and particles.
    case Additive
    /// Separate equations for the color and alpha channels.
    case Custom(BlendComponent, BlendComponent)

    /// The C blend state, or `None` to replace.
    var cValue: raw.WGPUBlendState? {
        let alphaOver = BlendComponent(source: .One, destination: .OneMinusSrcAlpha);
        match self {
            .Replace => .None,
            .AlphaBlending => .Some(raw.WGPUBlendState(
                color: BlendComponent(source: .SrcAlpha, destination: .OneMinusSrcAlpha).cValue,
                alpha: alphaOver.cValue
            )),
            .PremultipliedAlpha => .Some(raw.WGPUBlendState(color: alphaOver.cValue, alpha: alphaOver.cValue)),
            .Additive => .Some(raw.WGPUBlendState(
                color: BlendComponent(source: .One, destination: .One).cValue,
                alpha: BlendComponent(source: .One, destination: .One).cValue
            )),
            .Custom(color, alpha) => .Some(raw.WGPUBlendState(color: color.cValue, alpha: alpha.cValue))
        }
    }
}

/// A channel a color target writes.
public enum ColorChannel {
    case Red
    case Green
    case Blue
    case Alpha

    var bits: UInt64 {
        match self {
            .Red => raw.WGPUColorWriteMask_Red,
            .Green => raw.WGPUColorWriteMask_Green,
            .Blue => raw.WGPUColorWriteMask_Blue,
            .Alpha => raw.WGPUColorWriteMask_Alpha
        }
    }
}

/// One color output of a render pipeline.
public struct ColorTarget {
    public var format: TextureFormat
    public var blend: BlendMode
    /// The channels it writes; the rest keep their values.
    public var writeMask: [ColorChannel]

    public init(format format: TextureFormat, blend blend: BlendMode = .Replace, writeMask writeMask: [ColorChannel] = [.Red, .Green, .Blue, .Alpha]) {
        self.format = format;
        self.blend = blend;
        self.writeMask = writeMask;
    }
}

/// What a stencil test does to the stencil value.
public enum StencilOperation {
    case Keep
    case Zero
    /// Write the pass's `setStencilReference` value.
    case Replace
    case Invert
    case IncrementClamp
    case DecrementClamp
    case IncrementWrap
    case DecrementWrap

    var cValue: UInt32 {
        match self {
            .Keep => raw.WGPUStencilOperation_Keep,
            .Zero => raw.WGPUStencilOperation_Zero,
            .Replace => raw.WGPUStencilOperation_Replace,
            .Invert => raw.WGPUStencilOperation_Invert,
            .IncrementClamp => raw.WGPUStencilOperation_IncrementClamp,
            .DecrementClamp => raw.WGPUStencilOperation_DecrementClamp,
            .IncrementWrap => raw.WGPUStencilOperation_IncrementWrap,
            .DecrementWrap => raw.WGPUStencilOperation_DecrementWrap
        }
    }
}

/// The stencil test for one side of triangles: compare the reference with
/// the stored value, then update it by the outcome.
public struct StencilFace {
    public var compare: CompareFunction
    /// When the stencil test fails.
    public var fail: StencilOperation
    /// When the stencil test passes but the depth test fails.
    public var depthFail: StencilOperation
    /// When both pass.
    public var pass: StencilOperation

    public init(
        compare compare: CompareFunction = .Always,
        fail fail: StencilOperation = .Keep,
        depthFail depthFail: StencilOperation = .Keep,
        pass pass: StencilOperation = .Keep
    ) {
        self.compare = compare;
        self.fail = fail;
        self.depthFail = depthFail;
        self.pass = pass;
    }

    var cValue: raw.WGPUStencilFaceState {
        raw.WGPUStencilFaceState(
            compare: self.compare.cValue,
            failOp: self.fail.cValue,
            depthFailOp: self.depthFail.cValue,
            passOp: self.pass.cValue
        )
    }
}

/// A render pipeline's stencil test.
public struct StencilTest {
    public var front: StencilFace
    public var back: StencilFace
    /// The bits compared.
    public var readMask: Int64
    /// The bits written.
    public var writeMask: Int64

    public init(front front: StencilFace, back back: StencilFace, readMask readMask: Int64 = 255, writeMask writeMask: Int64 = 255) {
        self.front = front;
        self.back = back;
        self.readMask = readMask;
        self.writeMask = writeMask;
    }

    /// @name Both faces
    /// The same test for front and back faces.
    public init(both face: StencilFace, readMask readMask: Int64 = 255, writeMask writeMask: Int64 = 255) {
        self.front = face;
        self.back = face;
        self.readMask = readMask;
        self.writeMask = writeMask;
    }
}

/// A render pipeline's depth and stencil state.
public struct DepthTest {
    public var format: TextureFormat
    public var compare: CompareFunction
    /// Whether passing fragments write their depth.
    public var write: Bool
    /// The stencil test, for a format with stencil.
    public var stencil: StencilTest?
    /// A constant added to each fragment's depth, in units of the format's
    /// precision; shadow maps use it against acne.
    public var depthBias: Int64
    /// A bias scaled by the triangle's depth slope.
    public var depthBiasSlopeScale: Float32
    /// The largest bias allowed; 0 for no limit.
    public var depthBiasClamp: Float32

    public init(
        format format: TextureFormat = .Depth24Plus,
        compare compare: CompareFunction = .Less,
        write write: Bool = true,
        stencil stencil: StencilTest? = .None,
        depthBias depthBias: Int64 = 0,
        depthBiasSlopeScale depthBiasSlopeScale: Float32 = 0.0,
        depthBiasClamp depthBiasClamp: Float32 = 0.0
    ) {
        self.format = format;
        self.compare = compare;
        self.write = write;
        self.stencil = stencil;
        self.depthBias = depthBias;
        self.depthBiasSlopeScale = depthBiasSlopeScale;
        self.depthBiasClamp = depthBiasClamp;
    }

    var cValue: raw.WGPUDepthStencilState {
        let face = self.stencil.map { (s) in s.front.cValue } ?? StencilFace().cValue;
        let back = self.stencil.map { (s) in s.back.cValue } ?? StencilFace().cValue;
        raw.WGPUDepthStencilState(
            nextInChain: Pointer.nullPointer(),
            format: self.format.cValue,
            depthWriteEnabled: if self.write { raw.WGPUOptionalBool_True } else { raw.WGPUOptionalBool_False },
            depthCompare: self.compare.cValue,
            stencilFront: face,
            stencilBack: back,
            stencilReadMask: UInt32(from: self.stencil.map { (s) in s.readMask } ?? 0),
            stencilWriteMask: UInt32(from: self.stencil.map { (s) in s.writeMask } ?? 0),
            depthBias: Int32(from: self.depthBias),
            depthBiasSlopeScale: self.depthBiasSlopeScale,
            depthBiasClamp: self.depthBiasClamp
        )
    }
}

/// A vertex and fragment shader pair with its fixed-function state, ready
/// to draw.
public struct RenderPipeline: not Copyable {
    var handle: raw.WGPURenderPipeline

    /// The layout of bind group `group`, from the pipeline's layout or
    /// inferred from the shaders.
    public func bindGroupLayout(group: Int64) -> BindGroupLayout {
        BindGroupLayout(handle: unsafe raw.wgpuRenderPipelineGetBindGroupLayout(self.handle, UInt32(from: group)))
    }

    deinit {
        unsafe raw.wgpuRenderPipelineRelease(self.handle);
    }
}

extend Device {
    /// Create a `width` by `height` texture with `layers` array layers (or
    /// that depth, for `.D3`), `mipLevels` mip levels and `sampleCount`
    /// samples per texel (1, or 4 for multisampling).
    public func createTexture(
        width width: Int64,
        height height: Int64,
        format format: TextureFormat,
        usage usage: [TextureUsage],
        layers layers: Int64 = 1,
        dimension dimension: TextureDimension = .D2,
        mipLevels mipLevels: Int64 = 1,
        sampleCount sampleCount: Int64 = 1,
        label label: String = ""
    ) -> Texture {
        var bits: UInt64 = 0;
        for u in usage {
            bits = bits | u.bits;
        }
        let descriptor = raw.WGPUTextureDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            usage: bits,
            dimension: dimension.cValue,
            size: raw.WGPUExtent3D(width: UInt32(from: width), height: UInt32(from: height), depthOrArrayLayers: UInt32(from: layers)),
            format: format.cValue,
            mipLevelCount: UInt32(from: mipLevels),
            sampleCount: UInt32(from: sampleCount),
            viewFormatCount: 0,
            viewFormats: Pointer.nullPointer()
        );
        Texture(handle: unsafe raw.wgpuDeviceCreateTexture(self.handle, descriptor))
    }

    /// Copy tightly packed texel rows `data` into the whole of mip level
    /// `mipLevel` of layer `layer` of `texture` (a color format).
    public func write[T](texture texture: Texture, data data: [T], mipLevel mipLevel: Int64 = 0, layer layer: Int64 = 0) where T: FFISafe {
        let destination = texture.copyInfo(mipLevel, layer);
        let layout = raw.WGPUTexelCopyBufferLayout(
            offset: 0,
            bytesPerRow: UInt32(from: texture.levelWidth(mipLevel) * texture.format.bytesPerTexel),
            rowsPerImage: UInt32(from: texture.levelHeight(mipLevel))
        );
        let size = texture.levelExtent(mipLevel);
        let bytes = Layout.array[T](data.count).size;
        unsafe raw.wgpuQueueWriteTexture(self.queue, destination, data.asPointer().asRaw(), UInt64(from: bytes), layout, size);
    }

    /// Create a sampler. `filter` applies between texels and, unless
    /// `mipmapFilter` says otherwise, between mip levels. A `compare`
    /// sampler compares against a depth texture instead of reading it, for
    /// shadow maps. `anisotropy` above 1 (up to 16) sharpens textures seen
    /// at a slant, and needs `.Linear` filtering.
    public func createSampler(
        filter filter: FilterMode = .Linear,
        addressMode addressMode: AddressMode = .ClampToEdge,
        mipmapFilter mipmapFilter: FilterMode? = .None,
        compare compare: CompareFunction? = .None,
        anisotropy anisotropy: Int64 = 1,
        lodMinClamp lodMinClamp: Float32 = 0.0,
        lodMaxClamp lodMaxClamp: Float32 = 32.0,
        label label: String = ""
    ) -> Sampler {
        let descriptor = raw.WGPUSamplerDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            addressModeU: addressMode.cValue,
            addressModeV: addressMode.cValue,
            addressModeW: addressMode.cValue,
            magFilter: filter.cValue,
            minFilter: filter.cValue,
            mipmapFilter: (mipmapFilter ?? filter).mipValue,
            lodMinClamp: lodMinClamp,
            lodMaxClamp: lodMaxClamp,
            compare: match compare {
                .Some(c) => c.cValue,
                .None => raw.WGPUCompareFunction_Undefined
            },
            maxAnisotropy: UInt16(from: anisotropy)
        );
        Sampler(handle: unsafe raw.wgpuDeviceCreateSampler(self.handle, descriptor))
    }

    /// Create a render pipeline from `shader`'s `vertexEntry` and
    /// `fragmentEntry` functions, reading `buffers` and writing `targets`.
    /// With no `targets` (a depth-only pass, such as a shadow map) there is
    /// no fragment stage. Bind group layouts come from `layout`, or are
    /// inferred from the shader when it is `None`. `sampleCount` must match
    /// the attachments: 4 to draw into multisampled textures.
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
        sampleCount sampleCount: Int64 = 1,
        alphaToCoverage alphaToCoverage: Bool = false,
        layout layout: PipelineLayout? = .None,
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
        // One blend state per target (unused for `.Replace`), so each
        // target can point at its own.
        var blends: [raw.WGPUBlendState] = [];
        for t in targets {
            let unused = BlendComponent(source: .One, destination: .Zero).cValue;
            blends.append(t.blend.cValue ?? raw.WGPUBlendState(color: unused, alpha: unused));
        }
        var colorTargets: [raw.WGPUColorTargetState] = [];
        var t = 0;
        while t < targets.count {
            let target = targets(t);
            var mask: UInt64 = 0;
            for c in target.writeMask {
                mask = mask | c.bits;
            }
            colorTargets.append(raw.WGPUColorTargetState(
                nextInChain: Pointer.nullPointer(),
                format: target.format.cValue,
                blend: if target.blend.cValue.isSome() { blends.asPointer().offset(by: t) } else { Pointer.nullPointer() },
                writeMask: mask
            ));
            t = t + 1;
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
        var depthState = (depth ?? DepthTest()).cValue;
        let descriptor = raw.WGPURenderPipelineDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            layout: match layout {
                .Some(l) => l.handle,
                .None => RawPointer.nullPointer()
            },
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
            multisample: raw.WGPUMultisampleState(
                nextInChain: Pointer.nullPointer(),
                count: UInt32(from: sampleCount),
                mask: 4294967295,
                alphaToCoverageEnabled: if alphaToCoverage { 1 } else { 0 }
            ),
            fragment: if targets.count > 0 { Pointer(to: fragment) } else { Pointer.nullPointer() }
        );
        RenderPipeline(handle: unsafe raw.wgpuDeviceCreateRenderPipeline(self.handle, descriptor))
    }

    /// Read mip level `mipLevel` of `texture` (made with `.CopySrc`) back as
    /// tightly packed rows of bytes, waiting for the GPU. Color formats only.
    public func read(texture texture: Texture, mipLevel mipLevel: Int64 = 0) -> [UInt8] throws WgpuError {
        let padded = texture.paddedRowBytes(mipLevel);
        let height = texture.levelHeight(mipLevel);
        let row = texture.levelWidth(mipLevel) * texture.format.bytesPerTexel;
        let buffer = self.createBuffer(size: padded * height, usage: [.MapRead, .CopyDst], label: "texture readback");
        let encoder = self.createCommandEncoder();
        encoder.copy(texture: texture, to: buffer, mipLevel: mipLevel);
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

/// A depth-stencil buffer for a render pass, from
/// `TextureView.depthAttachment`. The view must outlive the pass.
public struct DepthAttachment {
    var cValue: raw.WGPURenderPassDepthStencilAttachment
}

/// Everything a render pass draws into and records: its attachments, and
/// optionally a query set for occlusion queries and timestamps written at
/// its start and end.
public struct RenderPassDescriptor {
    public var color: [ColorAttachment]
    public var depth: DepthAttachment?
    /// The query set `beginOcclusionQuery(index)` writes into.
    public var occlusionQueries: OcclusionQueries?
    public var timestamps: PassTimestamps?
    public var label: String

    public init(
        color color: [ColorAttachment],
        depth depth: DepthAttachment? = .None,
        occlusionQueries occlusionQueries: OcclusionQueries? = .None,
        timestamps timestamps: PassTimestamps? = .None,
        label label: String = ""
    ) {
        self.color = color;
        self.depth = depth;
        self.occlusionQueries = occlusionQueries;
        self.timestamps = timestamps;
        self.label = label;
    }
}

/// The type of an index buffer's indices.
public enum IndexFormat {
    case Uint16
    case Uint32

    var cValue: UInt32 {
        match self {
            .Uint16 => raw.WGPUIndexFormat_Uint16,
            .Uint32 => raw.WGPUIndexFormat_Uint32
        }
    }
}

extend CommandEncoder {
    /// Record a render pass drawing into `color`: `body` sets pipelines and
    /// draws.
    public func renderPass(color color: [ColorAttachment], body: (RenderPass) -> ()) {
        self.renderPass(RenderPassDescriptor(color: color), body)
    }

    /// Record a render pass drawing into `color` and testing against
    /// `depth`.
    public func renderPass(color color: [ColorAttachment], depth depth: DepthAttachment, body: (RenderPass) -> ()) {
        self.renderPass(RenderPassDescriptor(color: color, depth: .Some(depth)), body)
    }

    /// Record a render pass as `descriptor` describes it.
    public func renderPass(descriptor: RenderPassDescriptor, body: (RenderPass) -> ()) {
        var attachments: [raw.WGPURenderPassColorAttachment] = [];
        for c in descriptor.color {
            attachments.append(c.cValue);
        }
        var depthRaw = match descriptor.depth {
            .Some(d) => d.cValue,
            .None => raw.WGPURenderPassDepthStencilAttachment(
                view: RawPointer.nullPointer(),
                depthLoadOp: 0, depthStoreOp: 0, depthClearValue: 0.0, depthReadOnly: 0,
                stencilLoadOp: 0, stencilStoreOp: 0, stencilClearValue: 0, stencilReadOnly: 0
            )
        };
        var timestamps = match descriptor.timestamps {
            .Some(t) => t.cValue,
            .None => raw.WGPUPassTimestampWrites(querySet: RawPointer.nullPointer(), beginningOfPassWriteIndex: 0, endOfPassWriteIndex: 0)
        };
        let cDescriptor = raw.WGPURenderPassDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(descriptor.label),
            colorAttachmentCount: UInt64(from: attachments.count),
            colorAttachments: attachments.asPointer(),
            depthStencilAttachment: if descriptor.depth.isSome() { Pointer(to: depthRaw) } else { Pointer.nullPointer() },
            occlusionQuerySet: match descriptor.occlusionQueries {
                .Some(q) => q.handle,
                .None => RawPointer.nullPointer()
            },
            timestampWrites: if descriptor.timestamps.isSome() { Pointer(to: timestamps).asRaw() } else { RawPointer.nullPointer() }
        );
        let pass = RenderPass(handle: unsafe raw.wgpuCommandEncoderBeginRenderPass(self.handle, cDescriptor));
        body(pass);
        unsafe raw.wgpuRenderPassEncoderEnd(pass.handle);
    }

    /// Copy mip level `mipLevel` of `texture` (a color format, made with
    /// `.CopySrc`) into `buffer`, rows `texture.paddedRowBytes(mipLevel)`
    /// apart.
    public func copy(texture texture: Texture, to buffer: Buffer, mipLevel mipLevel: Int64 = 0, layer layer: Int64 = 0) {
        let source = texture.copyInfo(mipLevel, layer);
        let destination = raw.WGPUTexelCopyBufferInfo(
            layout: raw.WGPUTexelCopyBufferLayout(
                offset: 0,
                bytesPerRow: UInt32(from: texture.paddedRowBytes(mipLevel)),
                rowsPerImage: UInt32(from: texture.levelHeight(mipLevel))
            ),
            buffer: buffer.handle
        );
        unsafe raw.wgpuCommandEncoderCopyTextureToBuffer(self.handle, source, destination, texture.levelExtent(mipLevel));
    }

    /// Copy `buffer`, rows `bytesPerRow` apart (a multiple of 256), into mip
    /// level `mipLevel` of layer `layer` of `texture`.
    public func copy(buffer buffer: Buffer, toTexture texture: Texture, bytesPerRow bytesPerRow: Int64, mipLevel mipLevel: Int64 = 0, layer layer: Int64 = 0) {
        let source = raw.WGPUTexelCopyBufferInfo(
            layout: raw.WGPUTexelCopyBufferLayout(
                offset: 0,
                bytesPerRow: UInt32(from: bytesPerRow),
                rowsPerImage: UInt32(from: texture.levelHeight(mipLevel))
            ),
            buffer: buffer.handle
        );
        unsafe raw.wgpuCommandEncoderCopyBufferToTexture(self.handle, source, texture.copyInfo(mipLevel, layer), texture.levelExtent(mipLevel));
    }

    /// Copy mip level `mipLevel` of layer `fromLayer` of `source` to the
    /// same level of layer `toLayer` of `destination`, which must match its
    /// size and format.
    public func copy(texture source: Texture, toTexture destination: Texture, mipLevel mipLevel: Int64 = 0, fromLayer fromLayer: Int64 = 0, toLayer toLayer: Int64 = 0) {
        unsafe raw.wgpuCommandEncoderCopyTextureToTexture(
            self.handle,
            source.copyInfo(mipLevel, fromLayer),
            destination.copyInfo(mipLevel, toLayer),
            source.levelExtent(mipLevel)
        );
    }
}

/// The arguments `drawIndirect` reads from a buffer, laid out as WebGPU
/// expects.
public struct DrawIndirectArgs: FFISafe {
    var vertexCount: UInt32
    var instanceCount: UInt32
    var firstVertex: UInt32
    var firstInstance: UInt32

    public init(vertices vertices: Int64, instances instances: Int64 = 1, firstVertex firstVertex: Int64 = 0, firstInstance firstInstance: Int64 = 0) {
        self.vertexCount = UInt32(from: vertices);
        self.instanceCount = UInt32(from: instances);
        self.firstVertex = UInt32(from: firstVertex);
        self.firstInstance = UInt32(from: firstInstance);
    }
}

/// The arguments `drawIndexedIndirect` reads from a buffer.
public struct DrawIndexedIndirectArgs: FFISafe {
    var indexCount: UInt32
    var instanceCount: UInt32
    var firstIndex: UInt32
    var baseVertex: Int32
    var firstInstance: UInt32

    public init(indices indices: Int64, instances instances: Int64 = 1, firstIndex firstIndex: Int64 = 0, baseVertex baseVertex: Int64 = 0, firstInstance firstInstance: Int64 = 0) {
        self.indexCount = UInt32(from: indices);
        self.instanceCount = UInt32(from: instances);
        self.firstIndex = UInt32(from: firstIndex);
        self.baseVertex = Int32(from: baseVertex);
        self.firstInstance = UInt32(from: firstInstance);
    }
}

/// `offsets` as the C `uint32_t` array dynamic offsets are passed in.
func dynamicOffsetArray(offsets: [Int64]) -> [UInt32] {
    var out: [UInt32] = [];
    for o in offsets {
        out.append(UInt32(from: o));
    }
    out
}

/// The byte count of vertex or index buffer data from `offset`: `size`, or
/// the rest of `buffer`.
func rangeSize(buffer: Buffer, offset: Int64, size: Int64?) -> UInt64 {
    UInt64(from: size ?? (buffer.size - offset))
}

/// A render pass being recorded.
public struct RenderPass: not Copyable {
    var handle: raw.WGPURenderPassEncoder

    /// Use `pipeline` for the next draws.
    public func setPipeline(pipeline: RenderPipeline) {
        unsafe raw.wgpuRenderPassEncoderSetPipeline(self.handle, pipeline.handle);
    }

    /// Bind `bindGroup` at `index`. A group with dynamic buffers takes one
    /// byte offset per dynamic binding, in binding order.
    public func setBindGroup(index: Int64, bindGroup: BindGroup, dynamicOffsets dynamicOffsets: [Int64] = []) {
        let offsets = dynamicOffsetArray(dynamicOffsets);
        unsafe raw.wgpuRenderPassEncoderSetBindGroup(self.handle, UInt32(from: index), bindGroup.handle, UInt64(from: offsets.count), offsets.asPointer().asRaw());
    }

    /// Read vertex buffer `slot` from `buffer`, starting `offset` bytes in,
    /// for `size` bytes or the rest of the buffer.
    public func setVertexBuffer(slot: Int64, buffer: Buffer, offset offset: Int64 = 0, size size: Int64? = .None) {
        unsafe raw.wgpuRenderPassEncoderSetVertexBuffer(self.handle, UInt32(from: slot), buffer.handle, UInt64(from: offset), rangeSize(buffer, offset, size));
    }

    /// Read indices from `buffer`, starting `offset` bytes in.
    public func setIndexBuffer(buffer: Buffer, format format: IndexFormat = .Uint32, offset offset: Int64 = 0, size size: Int64? = .None) {
        unsafe raw.wgpuRenderPassEncoderSetIndexBuffer(self.handle, buffer.handle, format.cValue, UInt64(from: offset), rangeSize(buffer, offset, size));
    }

    /// Map clip space onto the given rectangle, in pixels, and depth onto
    /// `minDepth` to `maxDepth`.
    public func setViewport(x: Float32, y: Float32, width: Float32, height: Float32, minDepth minDepth: Float32 = 0.0, maxDepth maxDepth: Float32 = 1.0) {
        unsafe raw.wgpuRenderPassEncoderSetViewport(self.handle, x, y, width, height, minDepth, maxDepth);
    }

    /// Discard fragments outside the given rectangle, in pixels.
    public func setScissor(x: Int64, y: Int64, width: Int64, height: Int64) {
        unsafe raw.wgpuRenderPassEncoderSetScissorRect(self.handle, UInt32(from: x), UInt32(from: y), UInt32(from: width), UInt32(from: height));
    }

    /// The value stencil tests compare against and `.Replace` writes.
    public func setStencilReference(reference: Int64) {
        unsafe raw.wgpuRenderPassEncoderSetStencilReference(self.handle, UInt32(from: reference));
    }

    /// The color `BlendFactor.Constant` uses.
    public func setBlendConstant(color: Color) {
        unsafe raw.wgpuRenderPassEncoderSetBlendConstant(self.handle, color.cValue);
    }

    /// Set push constants (see `Feature.PushConstants`) for `stages` from
    /// `value`, starting `offset` bytes in.
    public func setPushConstants[T](stages stages: [ShaderStage], value value: T, offset offset: Int64 = 0) where T: FFISafe {
        let size = Layout.of[T]().size;
        unsafe raw.wgpuRenderPassEncoderSetPushConstants(self.handle, stageBits(stages), UInt32(from: offset), UInt32(from: size), Pointer(to: value).asRaw());
    }

    /// Draw `vertices` vertices, `instances` times.
    public func draw(vertices vertices: Int64, instances instances: Int64 = 1, firstVertex firstVertex: Int64 = 0, firstInstance firstInstance: Int64 = 0) {
        unsafe raw.wgpuRenderPassEncoderDraw(self.handle, UInt32(from: vertices), UInt32(from: instances), UInt32(from: firstVertex), UInt32(from: firstInstance));
    }

    /// Draw `indices` indices from the index buffer, `instances` times,
    /// adding `baseVertex` to each index.
    public func drawIndexed(indices indices: Int64, instances instances: Int64 = 1, firstIndex firstIndex: Int64 = 0, baseVertex baseVertex: Int64 = 0, firstInstance firstInstance: Int64 = 0) {
        unsafe raw.wgpuRenderPassEncoderDrawIndexed(self.handle, UInt32(from: indices), UInt32(from: instances), UInt32(from: firstIndex), Int32(from: baseVertex), UInt32(from: firstInstance));
    }

    /// Draw with a `DrawIndirectArgs` read from `buffer` (made with
    /// `.Indirect`) at `offset`, so the GPU can decide what to draw.
    public func drawIndirect(buffer: Buffer, offset offset: Int64 = 0) {
        unsafe raw.wgpuRenderPassEncoderDrawIndirect(self.handle, buffer.handle, UInt64(from: offset));
    }

    /// Draw with a `DrawIndexedIndirectArgs` read from `buffer` at `offset`.
    public func drawIndexedIndirect(buffer: Buffer, offset offset: Int64 = 0) {
        unsafe raw.wgpuRenderPassEncoderDrawIndexedIndirect(self.handle, buffer.handle, UInt64(from: offset));
    }

    /// Count the samples the draws until `endOcclusionQuery` pass, into
    /// query `index` of the pass's occlusion query set.
    public func beginOcclusionQuery(index: Int64) {
        unsafe raw.wgpuRenderPassEncoderBeginOcclusionQuery(self.handle, UInt32(from: index));
    }

    public func endOcclusionQuery() {
        unsafe raw.wgpuRenderPassEncoderEndOcclusionQuery(self.handle);
    }

    /// Replay the commands recorded in `bundle`. The pass's pipeline, bind
    /// groups and buffers are unset afterwards.
    public func execute(bundle: RenderBundle) {
        var handles: [raw.WGPURenderBundle] = [bundle.handle];
        unsafe raw.wgpuRenderPassEncoderExecuteBundles(self.handle, 1, handles.asPointer());
    }

    deinit {
        unsafe raw.wgpuRenderPassEncoderRelease(self.handle);
    }
}
