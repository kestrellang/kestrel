module Aerie

import linalg.(Vec3, Mat4)

/// Half the width of the square around the player the shadow map covers,
/// in blocks.
let shadowReach: Float32 = 72.0;

/// The light at one time of day: where the sun is and how strong, the
/// sky's colors at the horizon and overhead, the sunlight's color, the
/// ambient light from the sky, and how much it is night (for stars).
struct Daylight {
    var sun: Vec3
    var strength: Float32
    var horizon: Vec3
    var zenith: Vec3
    var sunColor: Vec3
    var ambient: Vec3
    var night: Float32
    /// How much of the lairs' crimson gloom is mixed in, 0 to 1.
    var gloom: Float32

    /// `phase` runs 0 to 1 over a day: 0 is sunrise, 0.25 noon, 0.5 sunset,
    /// 0.75 midnight. The palette is soft pastel: a pale rose-cream horizon
    /// under baby blue by day, apricot at dusk, deep indigo at night. Over the
    /// lairs, `gloom` (0 to 1) turns it smoky crimson under a red sun.
    static func at(phase: Float32, gloom gloom: Float32) -> Daylight {
        let angle = phase * Float32.pi * 2.0;
        // Up is +y; the sun crosses the sky from east to west, tilted south.
        let up = angle.sin();
        // The sun moves in steps of a fifth of a degree, about one every 0.3 s
        // of play: each new direction re-rasterizes every shadow edge, so a
        // sun that turned every frame would make shadows crawl.
        let steps: Float32 = 1800.0;
        let stepped = (phase * steps).floor() / steps * Float32.pi * 2.0;
        let sun = Vec3(x: -stepped.cos(), y: -stepped.sin(), z: 0.35).normalized;
        let day = (up * 3.0 + 0.3).clamp(0.0, 1.0);
        // Dusk warms everything just as the sun nears the horizon.
        let dusk = (1.0 - (up * 4.0).abs()).clamp(0.0, 1.0) * (up * 4.0 + 1.5).clamp(0.0, 1.0);
        let horizon = blend(blend(Vec3(x: 0.16, y: 0.15, z: 0.32), Vec3(x: 0.93, y: 0.89, z: 0.92), day), Vec3(x: 1.00, y: 0.74, z: 0.60), dusk * 0.7);
        let zenith = blend(blend(Vec3(x: 0.04, y: 0.04, z: 0.15), Vec3(x: 0.52, y: 0.70, z: 0.96), day), Vec3(x: 0.46, y: 0.48, z: 0.80), dusk * 0.6);
        let sunColor = blend(Vec3(x: 1.00, y: 0.92, z: 0.90), Vec3(x: 1.00, y: 0.70, z: 0.52), dusk);
        // Cool sky light, so shadows read blue-grey.
        let ambient = blend(Vec3(x: 0.12, y: 0.13, z: 0.26), Vec3(x: 0.54, y: 0.54, z: 0.64), day);
        // The lairs' palette, mixed in by `gloom`.
        let g = gloom * 0.9;
        let lairHorizon = blend(Vec3(x: 0.12, y: 0.03, z: 0.06), Vec3(x: 0.40, y: 0.11, z: 0.12), day);
        let lairZenith = blend(Vec3(x: 0.03, y: 0.01, z: 0.05), Vec3(x: 0.10, y: 0.04, z: 0.10), day);
        let lairAmbient = blend(Vec3(x: 0.14, y: 0.06, z: 0.10), Vec3(x: 0.36, y: 0.22, z: 0.28), day);
        Daylight(
            sun: sun, strength: day,
            horizon: blend(horizon, lairHorizon, g), zenith: blend(zenith, lairZenith, g),
            sunColor: blend(sunColor, Vec3(x: 1.0, y: 0.50, z: 0.32), g), ambient: blend(ambient, lairAmbient, g),
            night: 1.0 - day, gloom: gloom
        )
    }

    /// The sun's view-projection: an orthographic box around `center`,
    /// looking along the sun. The box moves in whole shadow texels of the sun's own view, so
    /// shadows don't shimmer as the player walks: snapping in world axes
    /// wouldn't line up with the map's texels, since the sun looks at an angle.
    /// `mapSize` is the shadow map's size in texels.
    func shadowView(center: Vec3, mapSize mapSize: Int64) -> Mat4 {
        let sun = self.sun;
        let texel = shadowReach * 2.0 / Float32(from: mapSize);
        // A view that only turns to face along `sun`; `center` in its axes.
        let f = sun.normalized;
        let view = Mat4.lookAt(eye: Vec3(x: 0.0, y: 0.0, z: 0.0) - f, target: Vec3(x: 0.0, y: 0.0, z: 0.0), up: Vec3.unitY);
        let side = f.cross(Vec3.unitY).normalized;
        let up = side.cross(f);
        let x = (side.dot(center) / texel).floor() * texel;
        let y = (up.dot(center) / texel).floor() * texel;
        // Distance along the view, which looks down its -z: the eye sits one
        // unit behind the origin.
        let depth = f.dot(center) + 1.0;
        Mat4.orthographic(left: x - shadowReach, right: x + shadowReach, bottom: y - shadowReach, top: y + shadowReach, near: depth - 150.0, far: depth + 150.0) * view
    }
}

/// `a` moved toward `b` by `t`, 0 to 1.
func blend(a: Vec3, b: Vec3, t: Float32) -> Vec3 {
    a + (b - a) * t
}
