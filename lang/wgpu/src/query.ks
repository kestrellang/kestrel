// Query sets (occlusion counts and GPU timestamps) and render bundles
// (draw commands recorded once and replayed in many passes).

module wgpu

import wgpu.raw

// ===== Query sets =====

/// What a query set records.
public enum QueryType {
    /// How many samples passed the depth and stencil tests between a pass's
    /// `beginOcclusionQuery` and `endOcclusionQuery`.
    case Occlusion
    /// The GPU's clock, in nanoseconds, at the start or end of a pass. Needs
    /// `Feature.TimestampQuery`.
    case Timestamp

    var cValue: UInt32 {
        match self {
            .Occlusion => raw.WGPUQueryType_Occlusion,
            .Timestamp => raw.WGPUQueryType_Timestamp
        }
    }
}

/// A set of `count` query results on the GPU.
public struct QuerySet: not Copyable {
    var handle: raw.WGPUQuerySet

    public var count: Int64 { Int64(from: unsafe raw.wgpuQuerySetGetCount(self.handle)) }

    public var kind: QueryType {
        if unsafe raw.wgpuQuerySetGetType(self.handle) == raw.WGPUQueryType_Timestamp { .Timestamp } else { .Occlusion }
    }

    /// This set as a render pass's occlusion queries. The set must outlive
    /// the pass.
    public var occlusionQueries: OcclusionQueries { OcclusionQueries(handle: self.handle) }

    /// Write the pass's start time into query `beginning` and its end time
    /// into query `end`; leave either out with `None`. The set must outlive
    /// the pass.
    public func timestamps(beginning beginning: Int64? = .None, end end: Int64? = .None) -> PassTimestamps {
        PassTimestamps(cValue: raw.WGPUPassTimestampWrites(
            querySet: self.handle,
            beginningOfPassWriteIndex: match beginning {
                .Some(i) => UInt32(from: i),
                .None => raw.WGPU_U32_UNDEFINED
            },
            endOfPassWriteIndex: match end {
                .Some(i) => UInt32(from: i),
                .None => raw.WGPU_U32_UNDEFINED
            }
        ))
    }

    deinit {
        unsafe raw.wgpuQuerySetRelease(self.handle);
    }
}

/// An occlusion query set for `RenderPassDescriptor`, from
/// `QuerySet.occlusionQueries`.
public struct OcclusionQueries {
    var handle: raw.WGPUQuerySet
}

/// Where a pass writes its start and end times, from `QuerySet.timestamps`.
public struct PassTimestamps {
    var cValue: raw.WGPUPassTimestampWrites
}

extend Device {
    /// A set of `count` queries of `kind`.
    public func createQuerySet(kind kind: QueryType, count count: Int64, label label: String = "") -> QuerySet {
        let descriptor = raw.WGPUQuerySetDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(label),
            kind: kind.cValue,
            count: UInt32(from: count)
        );
        QuerySet(handle: unsafe raw.wgpuDeviceCreateQuerySet(self.handle, descriptor))
    }

    /// Every result in `querySet`, once the work recorded so far has run:
    /// sample counts for occlusion, nanoseconds for timestamps.
    public func readQueries(querySet: QuerySet) -> [UInt64] throws WgpuError {
        let size = querySet.count * 8;
        let resolved = self.createBuffer(size: size, usage: [.QueryResolve, .CopySrc], label: "query resolve");
        let readback = self.createBuffer(size: size, usage: [.MapRead, .CopyDst], label: "query readback");
        let encoder = self.createCommandEncoder();
        encoder.resolve(querySet, into: resolved);
        encoder.copy(from: resolved, to: readback, size: size);
        self.submit(encoder.finish());
        self.read(readback)
    }
}

extend CommandEncoder {
    /// Copy `count` results from query `first` of `querySet` (all of them
    /// by default) into `buffer`, made with `.QueryResolve`, as `UInt64`s
    /// starting at byte `offset` (a multiple of 256).
    public func resolve(querySet: QuerySet, into buffer: Buffer, first first: Int64 = 0, count count: Int64? = .None, offset offset: Int64 = 0) {
        let n = count ?? (querySet.count - first);
        unsafe raw.wgpuCommandEncoderResolveQuerySet(self.handle, querySet.handle, UInt32(from: first), UInt32(from: n), buffer.handle, UInt64(from: offset));
    }
}

// ===== Render bundles =====

/// The attachments a render bundle will be replayed into: their formats
/// and sample count must match the pass's.
public struct BundleTargets {
    public var colorFormats: [TextureFormat]
    public var depthFormat: TextureFormat?
    public var sampleCount: Int64
    /// Whether the bundle leaves depth (and stencil) unwritten, so it can
    /// run in a pass whose depth attachment is read-only.
    public var depthReadOnly: Bool
    public var label: String

    public init(
        colorFormats colorFormats: [TextureFormat],
        depthFormat depthFormat: TextureFormat? = .None,
        sampleCount sampleCount: Int64 = 1,
        depthReadOnly depthReadOnly: Bool = false,
        label label: String = ""
    ) {
        self.colorFormats = colorFormats;
        self.depthFormat = depthFormat;
        self.sampleCount = sampleCount;
        self.depthReadOnly = depthReadOnly;
        self.label = label;
    }
}

/// Recorded draw commands, for `RenderPass.execute`.
public struct RenderBundle: not Copyable {
    var handle: raw.WGPURenderBundle

    deinit {
        unsafe raw.wgpuRenderBundleRelease(self.handle);
    }
}

/// A render bundle being recorded: the drawing half of `RenderPass`.
public struct RenderBundleEncoder: not Copyable {
    var handle: raw.WGPURenderBundleEncoder

    public func setPipeline(pipeline: RenderPipeline) {
        unsafe raw.wgpuRenderBundleEncoderSetPipeline(self.handle, pipeline.handle);
    }

    public func setBindGroup(index: Int64, bindGroup: BindGroup, dynamicOffsets dynamicOffsets: [Int64] = []) {
        let offsets = dynamicOffsetArray(dynamicOffsets);
        unsafe raw.wgpuRenderBundleEncoderSetBindGroup(self.handle, UInt32(from: index), bindGroup.handle, UInt64(from: offsets.count), offsets.asPointer().asRaw());
    }

    public func setVertexBuffer(slot: Int64, buffer: Buffer, offset offset: Int64 = 0, size size: Int64? = .None) {
        unsafe raw.wgpuRenderBundleEncoderSetVertexBuffer(self.handle, UInt32(from: slot), buffer.handle, UInt64(from: offset), rangeSize(buffer, offset, size));
    }

    public func setIndexBuffer(buffer: Buffer, format format: IndexFormat = .Uint32, offset offset: Int64 = 0, size size: Int64? = .None) {
        unsafe raw.wgpuRenderBundleEncoderSetIndexBuffer(self.handle, buffer.handle, format.cValue, UInt64(from: offset), rangeSize(buffer, offset, size));
    }

    public func setPushConstants[T](stages stages: [ShaderStage], value value: T, offset offset: Int64 = 0) where T: FFISafe {
        let size = Layout.of[T]().size;
        unsafe raw.wgpuRenderBundleEncoderSetPushConstants(self.handle, stageBits(stages), UInt32(from: offset), UInt32(from: size), Pointer(to: value).asRaw());
    }

    public func draw(vertices vertices: Int64, instances instances: Int64 = 1, firstVertex firstVertex: Int64 = 0, firstInstance firstInstance: Int64 = 0) {
        unsafe raw.wgpuRenderBundleEncoderDraw(self.handle, UInt32(from: vertices), UInt32(from: instances), UInt32(from: firstVertex), UInt32(from: firstInstance));
    }

    public func drawIndexed(indices indices: Int64, instances instances: Int64 = 1, firstIndex firstIndex: Int64 = 0, baseVertex baseVertex: Int64 = 0, firstInstance firstInstance: Int64 = 0) {
        unsafe raw.wgpuRenderBundleEncoderDrawIndexed(self.handle, UInt32(from: indices), UInt32(from: instances), UInt32(from: firstIndex), Int32(from: baseVertex), UInt32(from: firstInstance));
    }

    public func drawIndirect(buffer: Buffer, offset offset: Int64 = 0) {
        unsafe raw.wgpuRenderBundleEncoderDrawIndirect(self.handle, buffer.handle, UInt64(from: offset));
    }

    public func drawIndexedIndirect(buffer: Buffer, offset offset: Int64 = 0) {
        unsafe raw.wgpuRenderBundleEncoderDrawIndexedIndirect(self.handle, buffer.handle, UInt64(from: offset));
    }

    deinit {
        unsafe raw.wgpuRenderBundleEncoderRelease(self.handle);
    }
}

extend Device {
    /// Record a render bundle for passes drawing into `targets`: `body`
    /// sets pipelines and draws, once, and any number of passes can replay
    /// it with `execute`.
    public func createRenderBundle(targets: BundleTargets, body: (RenderBundleEncoder) -> ()) -> RenderBundle {
        var formats: [UInt32] = [];
        for f in targets.colorFormats {
            formats.append(f.cValue);
        }
        let descriptor = raw.WGPURenderBundleEncoderDescriptor(
            nextInChain: Pointer.nullPointer(),
            label: view(targets.label),
            colorFormatCount: UInt64(from: formats.count),
            colorFormats: formats.asPointer(),
            depthStencilFormat: match targets.depthFormat {
                .Some(f) => f.cValue,
                .None => raw.WGPUTextureFormat_Undefined
            },
            sampleCount: UInt32(from: targets.sampleCount),
            depthReadOnly: if targets.depthReadOnly { 1 } else { 0 },
            stencilReadOnly: if targets.depthReadOnly { 1 } else { 0 }
        );
        let encoder = RenderBundleEncoder(handle: unsafe raw.wgpuDeviceCreateRenderBundleEncoder(self.handle, descriptor));
        body(encoder);
        let finish = raw.WGPURenderBundleDescriptor(nextInChain: Pointer.nullPointer(), label: view(targets.label));
        RenderBundle(handle: unsafe raw.wgpuRenderBundleEncoderFinish(encoder.handle, finish))
    }
}
