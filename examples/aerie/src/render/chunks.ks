module Aerie

import wgpu.(Device, Buffer, BindGroup, RenderPipeline, RenderPass, ShaderModule, PipelineLayout, ColorTarget, VertexBufferLayout, DepthTest)
import linalg.(Vec3, Vec4)

// Every chunk's faces share one big vertex buffer, and every chunk's data
// (where it sits, the lights that reach it) one storage buffer, indexed by
// a slot number carried in each face. So drawing needs no per-chunk bind
// groups, and chunks that sit next to each other in the buffer draw in one
// call.

/// A run of faces in the shared face buffer, counted in faces.
struct FaceRange {
    var start: Int64
    var count: Int64

    static let empty: FaceRange = FaceRange(start: 0, count: 0)

    var end: Int64 { self.start + self.count }
}

/// A growable GPU buffer of faces, handed out in ranges, first fit.
struct FaceHeap: not Copyable {
    var buffer: Buffer
    /// How many faces the buffer holds.
    var capacity: Int64
    /// The unused ranges, by start; neighbors are always merged.
    var free: [FaceRange]

    static func create(device: Device, capacity capacity: Int64) -> FaceHeap {
        FaceHeap(buffer: FaceHeap.makeBuffer(device, capacity), capacity: capacity, free: [FaceRange(start: 0, count: capacity)])
    }

    static func makeBuffer(device: Device, capacity: Int64) -> Buffer {
        device.createBuffer(size: capacity * Layout.of[FaceInstance]().size, usage: [.Vertex, .CopyDst, .CopySrc], label: "chunk faces")
    }

    /// Room for `count` faces, growing the buffer when no gap fits.
    mutating func allocate(device: Device, count count: Int64) -> FaceRange {
        if let .Some(range) = self.take(count) {
            return range;
        }
        self.grow(device, atLeast: count);
        self.take(count) ?? fatalError("the face buffer grew but has no room")
    }

    /// The first gap that fits `count` faces, cut down to size.
    mutating func take(count: Int64) -> FaceRange? {
        for i in 0..<self.free.count {
            let gap = self.free(i);
            if gap.count < count {
                continue;
            }
            if gap.count == count {
                let _ = self.free.remove(at: i);
            } else {
                self.free(i) = FaceRange(start: gap.start + count, count: gap.count - count);
            }
            return .Some(FaceRange(start: gap.start, count: count));
        }
        .None
    }

    /// Give `range` back, merging it with the gaps either side.
    mutating func release(range: FaceRange) {
        guard range.count > 0 else {
            return;
        }
        var i: Int64 = 0;
        while i < self.free.count and self.free(i).start < range.start {
            i = i + 1;
        }
        var merged = range;
        if i < self.free.count and self.free(i).start == merged.end {
            merged.count = merged.count + self.free.remove(at: i).count;
        }
        if i > 0 and self.free(i - 1).end == merged.start {
            let before = self.free.remove(at: i - 1);
            merged = FaceRange(start: before.start, count: before.count + merged.count);
            i = i - 1;
        }
        self.free.insert(merged, at: i);
    }

    /// Double the buffer (or more, to fit `atLeast` faces), copying the
    /// faces already in it on the GPU.
    mutating func grow(device: Device, atLeast atLeast: Int64) {
        let old = self.capacity;
        let capacity = if old * 2 >= old + atLeast { old * 2 } else { old + atLeast };
        let bigger = FaceHeap.makeBuffer(device, capacity);
        let encoder = device.createCommandEncoder();
        encoder.copy(from: self.buffer, to: bigger, size: old * Layout.of[FaceInstance]().size);
        device.submit(encoder.finish());
        self.buffer = bigger;
        self.capacity = capacity;
        self.release(FaceRange(start: old, count: capacity - old));
    }
}

/// Where a chunk's faces are in the shared buffer, and its data slot.
struct ChunkMesh {
    var key: Int64
    var cx: Int64
    var cy: Int64
    var cz: Int64
    var slot: Int64
    /// Solid and alpha-tested faces, then see-through ones.
    var opaque: FaceRange
    var glass: FaceRange

    /// The chunk's corner with the lowest coordinates, in blocks.
    var low: Vec3 {
        let size = Float32(from: chunkSize);
        Vec3(x: Float32(from: self.cx) * size, y: Float32(from: self.cy) * size, z: Float32(from: self.cz) * size)
    }
}

/// The chunk meshes to draw in a frame, as face ranges.
struct DrawList {
    var opaque: [FaceRange]
    /// Chunks inside the sun's view, for the shadow map.
    var casters: [FaceRange]
    /// Visible glass, far chunks first.
    var glass: [FaceRange]
    /// How many chunks passed the camera's frustum.
    var visible: Int64
}

/// How many chunks can be loaded at once: every column out to the largest
/// view radius, plus the ring kept around it before unloading.
let maxChunkSlots: Int64 = (2 * (maxViewRadius + 2) + 1) * (2 * (maxViewRadius + 2) + 1) * worldChunksY;
/// The shader's `ChunkData`: the origin with the light count in w, then
/// `maxLights` light positions, each with its color (`Block.glow`) in w.
let chunkDataVectors: Int64 = maxLights + 1;
/// A face's chunk slot sits in the top half of `FaceInstance.shade`.
let slotShift: Int64 = 16;

/// The world's blocks: their pipelines, the shared face and chunk data
/// buffers, and which chunks are loaded where.
struct ChunkPass: not Copyable {
    var pipeline: RenderPipeline
    /// Glass blends over the opaque world.
    var glassPipeline: RenderPipeline
    /// Depth only, as the sun sees the blocks.
    var shadowPipeline: RenderPipeline
    var faces: FaceHeap
    /// Every chunk's `ChunkData`, by slot.
    var chunkData: Buffer
    /// Bind group 1 of the block and shadow pipelines: `chunkData`.
    var group: BindGroup
    var freeSlots: [Int64]
    var meshes: [ChunkMesh]
    /// Columns that have been meshed, by `chunkKey(cx, 0, cz)`, whether or
    /// not any of their chunks had faces.
    var meshedColumns: Dictionary[Int64, Bool]

    /// `layout` and `shadowLayout` are the block and shadow pipelines'
    /// layouts; group 1 of each is the chunk data.
    static func create(device: Device, shader: ShaderModule, target target: DrawTarget, layout layout: PipelineLayout, shadowLayout shadowLayout: PipelineLayout) -> ChunkPass {
        let buffers = [VertexBufferLayout(packed: [.Uint32, .Uint32], stepMode: .Instance)];
        let pipeline = device.createRenderPipeline(
            shader: shader,
            targets: [ColorTarget(format: target.format)],
            buffers: buffers,
            cullMode: .Back,
            depth: .Some(DepthTest(format: depthFormat)),
            sampleCount: target.samples,
            layout: .Some(layout.share()),
            label: "blocks"
        );
        // Tested against the opaque world's depth but not written, so
        // glass behind glass still shows.
        let glassPipeline = device.createRenderPipeline(
            shader: shader,
            targets: [ColorTarget(format: target.format, blend: .AlphaBlending)],
            buffers: buffers,
            fragmentEntry: "fs_glass",
            cullMode: .Back,
            depth: .Some(DepthTest(format: depthFormat, write: false)),
            sampleCount: target.samples,
            layout: .Some(layout.share()),
            label: "glass"
        );
        // The bias keeps lit faces from shadowing themselves.
        let shadowPipeline = device.createRenderPipeline(
            shader: shader,
            targets: [],
            buffers: buffers,
            vertexEntry: "vs_shadow",
            depth: .Some(DepthTest(format: shadowFormat, depthBias: 2, depthBiasSlopeScale: 2.0)),
            layout: .Some(shadowLayout.share()),
            label: "shadow"
        );
        let chunkData = device.createBuffer(size: maxChunkSlots * chunkDataVectors * Layout.of[Vec4]().size, usage: [.Storage, .CopyDst], label: "chunk data");
        let group = device.createBindGroup(layout: layout.bindGroupLayout(1), entries: [chunkData.binding(0)]);
        // Slot 0 is handed out first.
        var freeSlots: [Int64] = [];
        for i in 0..<maxChunkSlots {
            freeSlots.append(maxChunkSlots - 1 - i);
        }
        ChunkPass(
            pipeline: pipeline, glassPipeline: glassPipeline, shadowPipeline: shadowPipeline,
            faces: FaceHeap.create(device, capacity: 1 << 20),
            chunkData: chunkData, group: group, freeSlots: freeSlots,
            meshes: [], meshedColumns: Dictionary[Int64, Bool]()
        )
    }

    /// Whether column (cx, cz) has been meshed.
    func hasColumn(cx: Int64, cz: Int64) -> Bool {
        self.meshedColumns.contains(chunkKey(cx, 0, cz))
    }

    mutating func markColumn(cx: Int64, cz: Int64) {
        let _ = self.meshedColumns.insert(chunkKey(cx, 0, cz), true);
    }

    /// Draw `faces` for chunk (cx, cy, cz), lit by `lights`, replacing any
    /// it had.
    mutating func upload(device: Device, cx: Int64, cy: Int64, cz: Int64, faces: ChunkFaces, lights: [Vec4]) {
        let key = chunkKey(cx, cy, cz);
        self.remove(where: { (k: Int64, x: Int64, z: Int64) in k == key });
        guard faces.count > 0 else {
            return;
        }
        let slot = self.freeSlots.popLast() ?? fatalError("more chunks loaded than maxChunkSlots");
        let size = Float32(from: chunkSize);
        var data = [Vec4(x: Float32(from: cx) * size, y: Float32(from: cy) * size, z: Float32(from: cz) * size, w: Float32(from: lights.count))];
        data.append(contentsOf: lights);
        while data.count < chunkDataVectors {
            data.append(Vec4(x: 0.0, y: 0.0, z: 0.0, w: 0.0));
        }
        device.write(self.chunkData, data: data, offset: slot * chunkDataVectors * Layout.of[Vec4]().size);
        let opaque = self.place(device, faces.opaque, slot: slot);
        let glass = self.place(device, faces.glass, slot: slot);
        self.meshes.append(ChunkMesh(key: key, cx: cx, cy: cy, cz: cz, slot: slot, opaque: opaque, glass: glass));
    }

    /// Copy `faces` into the shared buffer, each tagged with `slot`.
    mutating func place(device: Device, faces: [FaceInstance], slot slot: Int64) -> FaceRange {
        guard faces.count > 0 else {
            return FaceRange.empty;
        }
        let range = self.faces.allocate(device, count: faces.count);
        let tag = UInt32(from: slot) << slotShift;
        var tagged: [FaceInstance] = [];
        tagged.reserveCapacity(faces.count);
        for face in faces {
            tagged.append(FaceInstance(packed: face.packed, shade: face.shade | tag));
        }
        device.write(self.faces.buffer, data: tagged, offset: range.start * Layout.of[FaceInstance]().size);
        range
    }

    /// Drop the meshes `test` picks, given each one's key and chunk x and
    /// z, freeing their faces and slots.
    mutating func remove(where test: (Int64, Int64, Int64) -> Bool) {
        var kept: [ChunkMesh] = [];
        for m in self.meshes {
            if not test(m.key, m.cx, m.cz) {
                kept.append(m);
                continue;
            }
            self.faces.release(m.opaque);
            self.faces.release(m.glass);
            self.freeSlots.append(m.slot);
        }
        self.meshes = kept;
    }

    /// Forget meshed columns more than `keep` columns from (pcx, pcz), and
    /// their meshes.
    mutating func unload(pcx: Int64, pcz: Int64, keep keep: Int64) {
        let far = { (x: Int64, z: Int64) in (x - pcx).abs() > keep or (z - pcz).abs() > keep };
        self.remove(where: { (key: Int64, x: Int64, z: Int64) in far(x, z) });
        self.meshedColumns.retain(where: { (key: Int64, done: Bool) in
            let (x, z) = columnOfKey(key);
            not far(x, z)
        });
    }

    /// The chunks inside the camera's `view` and the sun's `shadow`
    /// frustums, with the glassy ones sorted far from `eye` first.
    func cull(view: Frustum, shadow shadow: Frustum, eye eye: Vec3) -> DrawList {
        let size = Float32(from: chunkSize);
        var list = DrawList(opaque: [], casters: [], glass: [], visible: 0);
        // Visible glass, by squared distance to its chunk's middle.
        var glassy: [(Float32, FaceRange)] = [];
        for m in self.meshes {
            let low = m.low;
            let high = low + Vec3(splat: size);
            if shadow.contains(low, high) {
                list.casters.append(m.opaque);
            }
            if not view.contains(low, high) {
                continue;
            }
            list.visible = list.visible + 1;
            list.opaque.append(m.opaque);
            if m.glass.count > 0 {
                glassy.append(((low + Vec3(splat: size * 0.5) - eye).lengthSquared, m.glass));
            }
        }
        // Blending needs back to front: far chunks first. Faces within a
        // chunk aren't sorted, which only shows where glass overlaps glass.
        glassy.sort(by: { (a: (Float32, FaceRange), b: (Float32, FaceRange)) in a.0 > b.0 });
        for (_, range) in glassy {
            list.glass.append(range);
        }
        list.opaque = mergedRuns(sortedByStart(list.opaque));
        list.casters = mergedRuns(sortedByStart(list.casters));
        list
    }

    /// The opaque world into the shadow map; `uniforms` is bind group 0.
    func drawShadows(pass: RenderPass, list list: DrawList, uniforms uniforms: BindGroup) {
        pass.setPipeline(self.shadowPipeline);
        pass.setBindGroup(0, uniforms);
        self.drawRuns(pass, list.casters);
    }

    /// The opaque world; the caller has set bind group 0.
    func drawOpaque(pass: RenderPass, list list: DrawList) {
        pass.setPipeline(self.pipeline);
        self.drawRuns(pass, list.opaque);
    }

    /// Glass, over everything opaque.
    func drawGlass(pass: RenderPass, list list: DrawList) {
        pass.setPipeline(self.glassPipeline);
        self.drawRuns(pass, mergedRuns(list.glass));
    }

    /// One instanced quad draw per run of faces.
    func drawRuns(pass: RenderPass, runs: [FaceRange]) {
        pass.setBindGroup(1, self.group);
        pass.setVertexBuffer(0, self.faces.buffer);
        for run in runs {
            pass.draw(vertices: 6, instances: run.count, firstInstance: run.start);
        }
    }
}

/// `ranges` ordered by where they start in the buffer.
func sortedByStart(ranges: [FaceRange]) -> [FaceRange] {
    var sorted = ranges;
    sorted.sort(by: { (a: FaceRange, b: FaceRange) in a.start < b.start });
    sorted
}

/// `ranges` with each one that starts where the one before it ends merged
/// into it, keeping their order; empty ranges are dropped.
func mergedRuns(ranges: [FaceRange]) -> [FaceRange] {
    var runs: [FaceRange] = [];
    for r in ranges {
        if r.count == 0 {
            continue;
        }
        if let .Some(last) = runs.last {
            if last.end == r.start {
                runs(runs.count - 1) = FaceRange(start: last.start, count: last.count + r.count);
                continue;
            }
        }
        runs.append(r);
    }
    runs
}
