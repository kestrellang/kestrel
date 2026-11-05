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

    var index: UInt32 {
        match self {
            .Driftstone => 0, .Loam => 1, .MossTop => 2, .MossSide => 3,
            .Bark => 4, .Rings => 5, .Emberleaf => 6, .Cloudglass => 7,
            .Skyiron => 8, .Glowcap => 9, .Sailcloth => 10, .Featherthatch => 11,
            .Planks => 12
        }
    }

    static var count: Int64 { 13 }
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

    var id: UInt16 {
        match self {
            .Air => 0, .Driftstone => 1, .Loam => 2, .Mossloam => 3, .Emberwood => 4,
            .Emberleaf => 5, .Cloudglass => 6, .Skyiron => 7, .Glowcap => 8,
            .Sailcloth => 9, .Featherthatch => 10, .Planks => 11
        }
    }

    static func from(id: UInt16) -> Block {
        match id {
            1 => .Driftstone, 2 => .Loam, 3 => .Mossloam, 4 => .Emberwood,
            5 => .Emberleaf, 6 => .Cloudglass, 7 => .Skyiron, 8 => .Glowcap,
            9 => .Sailcloth, 10 => .Featherthatch, 11 => .Planks,
            _ => .Air
        }
    }

    var shape: Shape {
        match self {
            .Air => .Empty,
            .Emberleaf => .Clear,
            .Cloudglass => .Clear,
            .Glowcap => .Cross,
            _ => .Solid
        }
    }

    /// What the hotbar calls it.
    var name: String {
        match self {
            .Air => "Air", .Driftstone => "Driftstone", .Loam => "Loam", .Mossloam => "Mossloam",
            .Emberwood => "Emberwood", .Emberleaf => "Emberleaf", .Cloudglass => "Cloudglass",
            .Skyiron => "Skyiron", .Glowcap => "Glowcap", .Sailcloth => "Sailcloth",
            .Featherthatch => "Featherthatch", .Planks => "Planks"
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
        match self {
            .Glowcap => true,
            _ => false
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
