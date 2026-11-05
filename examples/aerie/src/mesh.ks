module Aerie

/// One visible face, drawn as an instance of a quad. The shader unpacks
/// `packed`: x, y and z in bits 0-14 (5 bits each), the face in bits 15-18
/// and the texture layer in bits 19-26. `shade` is reserved for ambient
/// occlusion in milestone 2.
struct FaceInstance: FFISafe {
    var packed: UInt32
    var shade: UInt32
}

func packFace(x: Int64, y: Int64, z: Int64, face: Face, layer: Layer) -> FaceInstance {
    let position = UInt32(from: x) | (UInt32(from: y) << 5) | (UInt32(from: z) << 10);
    FaceInstance(packed: position | (face.index << 15) | (layer.index << 19), shade: 0)
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

/// Every visible face in `chunk`. Faces between two solid blocks are
/// skipped; faces on the chunk's edge are kept, since there are no
/// neighbor chunks yet.
func meshChunk(chunk: Chunk) -> [FaceInstance] {
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
                            faces.append(packFace(x, y, z, face, block.layer(face)));
                        }
                    },
                    _ => {
                        for face in Face.cube {
                            let (dx, dy, dz) = face.step;
                            if faceVisible(block, chunk.block(x + dx, y + dy, z + dz)) {
                                faces.append(packFace(x, y, z, face, block.layer(face)));
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
