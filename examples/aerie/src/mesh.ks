module Aerie

/// One visible face, drawn as an instance of a quad. The shader unpacks
/// `packed`: x, y and z in bits 0-14 (5 bits each), the face in bits 15-18
/// and the texture layer in bits 19-26. `shade` holds each corner's ambient
/// occlusion, 0 (darkest) to 3, two bits per corner, and bit 8 says to split
/// the quad along its other diagonal.
struct FaceInstance: FFISafe {
    var packed: UInt32
    var shade: UInt32
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

func packFace(x: Int64, y: Int64, z: Int64, face: Face, layer: Layer, shade: UInt32) -> FaceInstance {
    let position = UInt32(from: x) | (UInt32(from: y) << 5) | (UInt32(from: z) << 10);
    FaceInstance(packed: position | (face.index << 15) | (layer.index << 19), shade: shade)
}

/// 1 if (x, y, z) holds a block that darkens corners next to it.
func occludes(chunk: Padded, x: Int64, y: Int64, z: Int64) -> Int64 {
    match chunk.block(x, y, z).shape {
        .Solid => 1,
        _ => 0
    }
}

/// Toward -1 or +1 along one axis from a corner's 0 or 1 offset.
func outward(offset: Int64) -> Int64 {
    if offset == 0 { -1 } else { 1 }
}

/// Ambient occlusion for each corner of `face` on the block at (x, y, z),
/// packed for `FaceInstance.shade`. A corner looks at the three cells that
/// touch it in the layer in front of the face: the two sides and the
/// diagonal. Two sides together fully darken it, whatever the diagonal is.
func faceShade(chunk: Padded, x: Int64, y: Int64, z: Int64, face: Face) -> UInt32 {
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
        let side1 = occludes(chunk, fx + s1x, fy + s1y, fz + s1z);
        let side2 = occludes(chunk, fx + s2x, fy + s2y, fz + s2z);
        let diagonal = occludes(chunk, fx + ax, fy + ay, fz + az);
        levels.append(if side1 == 1 and side2 == 1 { 0 } else { 3 - side1 - side2 - diagonal });
    }
    var shade: UInt32 = 0;
    var i: Int64 = 0;
    while i < 4 {
        shade = shade | (UInt32(from: levels(i)) << (i * 2));
        i = i + 1;
    }
    // Split along the brighter diagonal, so the darkening stays smooth
    // instead of showing a crease.
    if levels(0) + levels(2) < levels(1) + levels(3) {
        shade = shade | 256;
    }
    shade
}

/// Whether a face of `block` that looks at `neighbor` can be seen.
func faceVisible(block: Block, neighbor: Block) -> Bool {
    match neighbor.shape {
        .Solid => false,
        // Leaves and glass hide their own kind, so a canopy isn't a maze
        // of inner faces.
        .Clear => neighbor.id != block.id,
        _ => true
    }
}

/// Every visible face of the chunk in the middle of `chunk`. Faces between
/// two solid blocks are skipped, including across chunk edges, since the
/// padding holds the neighbors' bordering blocks.
func meshChunk(chunk: Padded) -> [FaceInstance] {
    var faces: [FaceInstance] = [];
    var y: Int64 = 0;
    while y < chunkSize {
        var z: Int64 = 0;
        while z < chunkSize {
            var x: Int64 = 0;
            while x < chunkSize {
                let block = chunk.block(x, y, z);
                match block.shape {
                    .Empty => {},
                    .Cross => {
                        for face in Face.cross {
                            faces.append(packFace(x, y, z, face, block.layer(face), 255));
                        }
                    },
                    _ => {
                        for face in Face.cube {
                            let (dx, dy, dz) = face.step;
                            if faceVisible(block, chunk.block(x + dx, y + dy, z + dz)) {
                                let shade = faceShade(chunk, x, y, z, face);
                                faces.append(packFace(x, y, z, face, block.layer(face), shade));
                            }
                        }
                    }
                }
                x = x + 1;
            }
            z = z + 1;
        }
        y = y + 1;
    }
    faces
}

/// `cubeCorners` as WGSL `vec3f(...)` entries, for the shader's table.
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
