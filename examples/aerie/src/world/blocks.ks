module Aerie

/// A texture layer in `assets/blocks.png`, one 16x16 tile each, top to
/// bottom. The order must match `TILES` in `tools/make_atlas.py`.
enum Layer {
    case Driftstone
    case Loam
    case MossTop
    case MossSide
    case Bark
    case Rings
    case Emberleaf
    case Cloudglass
    case Skyiron
    case Glowcap
    case Sailcloth
    case Featherthatch
    case Planks
    case Ashrock
    case ScorchTop
    case ScorchSide
    case Darkbrick
    case Embercore
    case Deadwood
    case DeadRings
    case Gloomleaf
    case Hexbloom
    // Birds (see birds.ks) and the cracks on a block being broken.
    case KestrelBack
    case KestrelBelly
    case KestrelWing
    case KestrelWingUnder
    case KestrelTips
    case KestrelHead
    case KestrelTail
    case Beak
    case VultureBody
    case VultureWing
    case VultureWingUnder
    case VultureHead
    case VultureRuff
    case Crack0

    var index: UInt32 {
        match self {
            .Driftstone => 0, .Loam => 1, .MossTop => 2, .MossSide => 3,
            .Bark => 4, .Rings => 5, .Emberleaf => 6, .Cloudglass => 7,
            .Skyiron => 8, .Glowcap => 9, .Sailcloth => 10, .Featherthatch => 11,
            .Planks => 12, .Ashrock => 13, .ScorchTop => 14, .ScorchSide => 15,
            .Darkbrick => 16, .Embercore => 17, .Deadwood => 18, .DeadRings => 19,
            .Gloomleaf => 20, .Hexbloom => 21,
            .KestrelBack => 22, .KestrelBelly => 23, .KestrelWing => 24, .KestrelWingUnder => 25,
            .KestrelTips => 26, .KestrelHead => 27, .KestrelTail => 28, .Beak => 29,
            .VultureBody => 30, .VultureWing => 31, .VultureWingUnder => 32,
            .VultureHead => 33, .VultureRuff => 34, .Crack0 => 35
        }
    }

    /// `index` as a float, for shaders that take layers in vectors.
    var number: Float32 {
        Float32(from: Int64(from: self.index))
    }

    /// The crack stages follow `Crack0` in the atlas.
    static var crackStages: Int64 { 5 }

    static var count: Int64 { 40 }
}

/// How a block is drawn.
enum Shape {
    /// Not drawn.
    case Empty
    /// A full cube that hides the faces of its neighbors.
    case Solid
    /// A full cube with holes or see-through texels: neighbors stay visible.
    case Clear
    /// Two crossed quads, for plants.
    case Cross
}

/// Block ids as stored in a chunk. Ids are stable: saves will store them.
enum Block {
    case Air
    case Driftstone
    case Loam
    case Mossloam
    case Emberwood
    case Emberleaf
    case Cloudglass
    case Skyiron
    case Glowcap
    case Sailcloth
    case Featherthatch
    case Planks
    // The evil lair islands.
    case Ashrock
    case Scorchloam
    case Darkbrick
    case Embercore
    case Deadwood
    case Gloomleaf
    case Hexbloom

    var id: UInt16 {
        match self {
            .Air => 0, .Driftstone => 1, .Loam => 2, .Mossloam => 3, .Emberwood => 4,
            .Emberleaf => 5, .Cloudglass => 6, .Skyiron => 7, .Glowcap => 8,
            .Sailcloth => 9, .Featherthatch => 10, .Planks => 11,
            .Ashrock => 12, .Scorchloam => 13, .Darkbrick => 14, .Embercore => 15,
            .Deadwood => 16, .Gloomleaf => 17, .Hexbloom => 18
        }
    }

    static func from(id: UInt16) -> Block {
        match id {
            1 => .Driftstone, 2 => .Loam, 3 => .Mossloam, 4 => .Emberwood,
            5 => .Emberleaf, 6 => .Cloudglass, 7 => .Skyiron, 8 => .Glowcap,
            9 => .Sailcloth, 10 => .Featherthatch, 11 => .Planks,
            12 => .Ashrock, 13 => .Scorchloam, 14 => .Darkbrick, 15 => .Embercore,
            16 => .Deadwood, 17 => .Gloomleaf, 18 => .Hexbloom,
            _ => .Air
        }
    }

    var shape: Shape {
        match self {
            .Air => .Empty,
            .Emberleaf => .Clear,
            .Gloomleaf => .Clear,
            .Cloudglass => .Clear,
            .Glowcap => .Cross,
            .Hexbloom => .Cross,
            _ => .Solid
        }
    }

    var isAir: Bool {
        match self {
            .Air => true,
            _ => false
        }
    }

    /// What the hotbar calls it.
    var name: String {
        match self {
            .Air => "Air", .Driftstone => "Driftstone", .Loam => "Loam", .Mossloam => "Mossloam",
            .Emberwood => "Emberwood", .Emberleaf => "Emberleaf", .Cloudglass => "Cloudglass",
            .Skyiron => "Skyiron", .Glowcap => "Glowcap", .Sailcloth => "Sailcloth",
            .Featherthatch => "Featherthatch", .Planks => "Planks",
            .Ashrock => "Ashrock", .Scorchloam => "Scorchloam", .Darkbrick => "Darkbrick",
            .Embercore => "Embercore", .Deadwood => "Deadwood", .Gloomleaf => "Gloomleaf",
            .Hexbloom => "Hexbloom"
        }
    }

    /// Drawn see-through, blended over what's behind it after the solid
    /// world, instead of alpha-tested.
    var translucent: Bool {
        match self {
            .Cloudglass => true,
            _ => false
        }
    }

    /// Whether the block glows, lighting what's around it.
    var emits: Bool {
        self.glow > 0.0
    }

    /// The color a glowing block lights its surroundings with: 1 for the
    /// glowcaps' cool cyan, 2 for the lairs' ember red, 0 for none. The
    /// shader picks the color from this (see `glow` there).
    var glow: Float32 {
        match self {
            .Glowcap => 1.0,
            .Embercore => 2.0,
            .Hexbloom => 2.0,
            _ => 0.0
        }
    }

    /// Seconds of holding break to knock the block out: plants pop at
    /// once, soil and leaves are quick, stone and metal take a while.
    var hardness: Float32 {
        match self {
            .Glowcap => 0.1, .Hexbloom => 0.1,
            .Emberleaf => 0.3, .Gloomleaf => 0.3,
            .Loam => 0.5, .Mossloam => 0.6, .Scorchloam => 0.6, .Cloudglass => 0.5,
            .Sailcloth => 0.5, .Featherthatch => 0.5,
            .Emberwood => 1.0, .Deadwood => 1.0, .Planks => 0.9,
            .Driftstone => 1.4, .Ashrock => 1.4, .Embercore => 1.6, .Darkbrick => 1.8,
            .Skyiron => 2.5,
            _ => 1.0
        }
    }

    /// Whether a face of this block that looks at `neighbor` can be seen.
    func showsFace(toward neighbor: Block) -> Bool {
        match neighbor.shape {
            .Solid => false,
            // Leaves and glass hide their own kind, so a canopy isn't a
            // maze of inner faces.
            .Clear => neighbor.id != self.id,
            _ => true
        }
    }

    /// The texture on `face` (see `Face`).
    func layer(face: Face) -> Layer {
        match self {
            .Mossloam => match face {
                .Up => .MossTop,
                .Down => .Loam,
                _ => .MossSide
            },
            .Emberwood => match face {
                .Up => .Rings,
                .Down => .Rings,
                _ => .Bark
            },
            .Loam => .Loam,
            .Emberleaf => .Emberleaf,
            .Cloudglass => .Cloudglass,
            .Skyiron => .Skyiron,
            .Glowcap => .Glowcap,
            .Sailcloth => .Sailcloth,
            .Featherthatch => .Featherthatch,
            .Planks => .Planks,
            .Ashrock => .Ashrock,
            .Scorchloam => match face {
                .Up => .ScorchTop,
                .Down => .Ashrock,
                _ => .ScorchSide
            },
            .Darkbrick => .Darkbrick,
            .Embercore => .Embercore,
            .Deadwood => match face {
                .Up => .DeadRings,
                .Down => .DeadRings,
                _ => .Deadwood
            },
            .Gloomleaf => .Gloomleaf,
            .Hexbloom => .Hexbloom,
            _ => .Driftstone
        }
    }
}

/// A cube face, or one side of a plant's crossed quads. The numbering is
/// shared with the shader's corner table.
enum Face {
    case East
    case West
    case Up
    case Down
    case South
    case North
    case Cross0
    case Cross1
    case Cross2
    case Cross3

    var index: UInt32 {
        match self {
            .East => 0, .West => 1, .Up => 2, .Down => 3, .South => 4, .North => 5,
            .Cross0 => 6, .Cross1 => 7, .Cross2 => 8, .Cross3 => 9
        }
    }

    /// The neighbor this face looks at, as (dx, dy, dz).
    var step: (Int64, Int64, Int64) {
        match self {
            .East => (1, 0, 0), .West => (-1, 0, 0),
            .Up => (0, 1, 0), .Down => (0, -1, 0),
            .South => (0, 0, 1), .North => (0, 0, -1),
            _ => (0, 0, 0)
        }
    }

    static var cube: [Face] { [.East, .West, .Up, .Down, .South, .North] }
    static var cross: [Face] { [.Cross0, .Cross1, .Cross2, .Cross3] }
}
