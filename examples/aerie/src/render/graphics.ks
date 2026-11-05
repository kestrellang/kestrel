module Aerie

import wgpu.(PresentMode, TextureFormat)

/// How much the renderer spends on looks. Lower settings are for slower
/// GPUs and the browser; they change edges and shadows, never what's
/// drawn.
enum Quality {
    /// No multisampling, a 1024 shadow map and a 3x3 shadow filter.
    case Low
    /// 4x multisampling, a 2048 shadow map and a 3x3 shadow filter.
    case Medium
    /// 4x multisampling, a 2048 shadow map and a 5x5 shadow filter.
    case High

    /// The quality called `name` (`low`, `medium` or `high`).
    static func named(name: String) -> Quality? {
        match name {
            "low" => .Some(.Low),
            "medium" => .Some(.Medium),
            "high" => .Some(.High),
            _ => .None
        }
    }

    /// Samples per pixel for multisampling; 1 for none.
    var samples: Int64 {
        match self { .Low => 1, _ => 4 }
    }

    /// The shadow map's size in texels.
    var shadowSize: Int64 {
        match self { .Low => 1024, _ => 2048 }
    }

    /// How many texels each side of a pixel the shadow filter averages:
    /// 1 for 3x3 taps, 2 for 5x5.
    var shadowFilter: Int64 {
        match self { .High => 2, _ => 1 }
    }
}

/// How the renderer should look and present, chosen at startup.
struct Graphics {
    var presentMode: PresentMode
    var quality: Quality
}

/// What the main pass draws into: the surface's format, multisampled
/// `samples` times. Every pipeline in the pass must match it.
struct DrawTarget {
    var format: TextureFormat
    var samples: Int64

    /// Whether shaders apply the sRGB curve themselves, for surfaces that
    /// don't (see surface.wgsl).
    var encode: Bool { not self.format.isSrgb }
}
