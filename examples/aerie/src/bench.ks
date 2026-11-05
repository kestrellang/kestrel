module Aerie

import sdl.(monotonicMs)

/// `AERIE_BENCH`: times generating and meshing without a window.
struct Bench {
    /// Generate the 3x3 columns around the spawn point `runs` times, then
    /// mesh the middle column `runs` times, and print the average per
    /// chunk.
    static func run(runs: Int64) {
        let generator = Generator(1);
        let (cx, cz) = columnOf(generator.spawnPoint());
        let start = unsafe monotonicMs();
        var world = World(1);
        for _ in 0..<runs {
            world = World(1);
            for i in 0..<9 {
                world.generateColumn(cx + i % 3 - 1, cz + i / 3 - 1);
            }
        }
        let built = unsafe monotonicMs();
        for _ in 0..<runs {
            for cy in 0..<worldChunksY {
                let _ = world.padded(cx, cy, cz);
            }
        }
        let padded = unsafe monotonicMs();
        var faces = 0;
        for _ in 0..<runs {
            faces = 0;
            for cy in 0..<worldChunksY {
                faces = faces + world.padded(cx, cy, cz).mesh().count;
            }
        }
        let meshed = unsafe monotonicMs();
        // Per chunk: 9 columns of 4 chunks generated, 4 chunks meshed.
        let generated = Float64(from: built - start) / Float64(from: runs * 9 * worldChunksY);
        let pad = Float64(from: padded - built) / Float64(from: runs * worldChunksY);
        let mesh = Float64(from: meshed - padded) / Float64(from: runs * worldChunksY);
        println("\(runs) runs, \(faces) faces in the spawn column");
        println("generate: \(generated) ms per chunk");
        println("pad:      \(pad) ms per chunk");
        println("mesh:     \(mesh) ms per chunk, padding included");
    }
}
