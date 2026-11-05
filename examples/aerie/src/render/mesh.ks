module Aerie

/// One visible face, drawn as an instance of a quad. The shader unpacks
/// `packed`: x, y and z in bits 0-14 (5 bits each), the face in bits 15-18
/// and the texture layer in bits 19-26. `shade` holds each corner's ambient
/// occlusion, 0 (darkest) to 3, two bits per corner; bit 8 says to split
/// the quad along its other diagonal, and bits 16-31 are the chunk's data
/// slot, set as it's uploaded (see `ChunkPass.place`).
struct FaceInstance: FFISafe {
    var packed: UInt32
    var shade: UInt32

    /// The `face` of the block at (x, y, z) in its chunk, drawn with
    /// `layer` and shaded by `shade`.
    static func pack(x: Int64, y: Int64, z: Int64, face: Face, layer: Layer, shade: UInt32) -> FaceInstance {
        let position = UInt32(from: x) | (UInt32(from: y) << 5) | (UInt32(from: z) << 10);
        FaceInstance(packed: position | (face.index << 15) | (layer.index << 19), shade: shade)
    }
}

/// The corners of each cube face, in the order the shader draws them:
/// counter-clockwise seen from outside, as offsets inside the unit cell.
/// The shader's corner table is generated from this, so the two can't
/// drift apart.
let cubeCorners: [[(Int64, Int64, Int64)]] = [
    [(1, 0, 1), (1, 0, 0), (1, 1, 0), (1, 1, 1)],
    [(0, 0, 0), (0, 0, 1), (0, 1, 1), (0, 1, 0)],
    [(0, 1, 1), (1, 1, 1), (1, 1, 0), (0, 1, 0)],
    [(0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1)],
    [(0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)],
    [(1, 0, 0), (0, 0, 0), (0, 1, 0), (1, 1, 0)]
];

/// A chunk's faces, split by how they're drawn.
struct ChunkFaces {
    /// Solid and alpha-tested faces, drawn first in any order.
    var opaque: [FaceInstance]
    /// See-through faces, blended after everything opaque.
    var glass: [FaceInstance]

    init() {
        self.opaque = [];
        self.glass = [];
    }

    var count: Int64 { self.opaque.count + self.glass.count }

    mutating func add(face: FaceInstance, glass glass: Bool) {
        if glass { self.glass.append(face); } else { self.opaque.append(face); }
    }
}

/// Plants' crossed quads have no corners to darken: every corner fully lit.
let unshaded: UInt32 = 255;

extend Padded {
    /// Every visible face of the chunk in the middle. Faces between two
    /// solid blocks are skipped, including across chunk edges, since the
    /// padding holds the neighbors' bordering blocks.
    func mesh() -> ChunkFaces {
        var faces = ChunkFaces();
        for i in 0..<(chunkSize * chunkSize * chunkSize) {
            let (x, y, z) = (i % chunkSize, i / (chunkSize * chunkSize), i / chunkSize % chunkSize);
            self.addFaces(x, y, z, into: faces);
        }
        faces
    }

    /// The visible faces of the block at (x, y, z).
    func addFaces(x: Int64, y: Int64, z: Int64, mutating into faces: ChunkFaces) {
        let block = self.block(x, y, z);
        match block.shape {
            .Empty => {},
            .Cross => {
                for face in Face.cross {
                    faces.add(FaceInstance.pack(x, y, z, face, block.layer(face), unshaded), glass: false);
                }
            },
            _ => {
                for face in Face.cube {
                    let (dx, dy, dz) = face.step;
                    if block.showsFace(toward: self.block(x + dx, y + dy, z + dz)) {
                        let shade = self.shade(x, y, z, face);
                        faces.add(FaceInstance.pack(x, y, z, face, block.layer(face), shade), glass: block.translucent);
                    }
                }
            }
        }
    }

    /// 1 if (x, y, z) holds a block that darkens corners next to it.
    func occludes(x: Int64, y: Int64, z: Int64) -> Int64 {
        match self.block(x, y, z).shape {
            .Solid => 1,
            _ => 0
        }
    }

    /// Ambient occlusion for each corner of `face` on the block at (x, y,
    /// z), packed for `FaceInstance.shade`. A corner looks at the three
    /// cells that touch it in the layer in front of the face: the two sides
    /// and the diagonal. Two sides together fully darken it, whatever the
    /// diagonal is.
    func shade(x: Int64, y: Int64, z: Int64, face: Face) -> UInt32 {
        let (nx, ny, nz) = face.step;
        let (fx, fy, fz) = (x + nx, y + ny, z + nz);
        var levels: [Int64] = [];
        for (cx, cy, cz) in cubeCorners(Int64(from: face.index)) {
            // Steps along the face's two in-plane axes; zero along the normal.
            let ax = if nx == 0 { outward(cx) } else { 0 };
            let ay = if ny == 0 { outward(cy) } else { 0 };
            let az = if nz == 0 { outward(cz) } else { 0 };
            // Split the in-plane step into its two axes.
            let (s1x, s1y, s1z) = if ax != 0 { (ax, 0, 0) } else { (0, ay, 0) };
            let (s2x, s2y, s2z) = if az != 0 { (0, 0, az) } else { (0, ay, 0) };
            let side1 = self.occludes(fx + s1x, fy + s1y, fz + s1z);
            let side2 = self.occludes(fx + s2x, fy + s2y, fz + s2z);
            let diagonal = self.occludes(fx + ax, fy + ay, fz + az);
            levels.append(if side1 == 1 and side2 == 1 { 0 } else { 3 - side1 - side2 - diagonal });
        }
        var shade: UInt32 = 0;
        for i in 0..<4 {
            shade = shade | (UInt32(from: levels(i)) << (i * 2));
        }
        // Split along the brighter diagonal, so the darkening stays smooth
        // instead of showing a crease.
        if levels(0) + levels(2) < levels(1) + levels(3) {
            shade = shade | 256;
        }
        shade
    }
}

/// Toward -1 or +1 along one axis from a corner's 0 or 1 offset.
func outward(offset: Int64) -> Int64 {
    if offset == 0 { -1 } else { 1 }
}

/// `cubeCorners` as WGSL `vec3f(...)` entries, for the shaders' CUBE_CORNERS.
func cubeCornersWGSL() -> String {
    var out = "";
    for face in cubeCorners {
        for (x, y, z) in face {
            out = out + "vec3f(\(x), \(y), \(z)), ";
        }
        out = out + "\n    ";
    }
    out
}
