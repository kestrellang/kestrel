module Aerie

import std.io.file.(readFileString)

// The shaders live in assets/shaders/*.wgsl. Each is loaded after a small
// header of constants the game decides (so they have one source of truth,
// here) and after surface.wgsl, which every fragment's output goes
// through.

/// The constants every shader may use. `encode` is whether to apply the
/// sRGB curve in the shader, for surfaces that don't (see surface.wgsl).
func shaderHeader(encode: Bool) -> String {
    "const ENCODE_SRGB = \(encode);
const MAX_LIGHTS = \(maxLights);
const LIGHT_RADIUS = \(lightRadius);
const GLOWCAP_LAYER = \(Layer.Glowcap.index)u;
const EMBERCORE_LAYER = \(Layer.Embercore.index)u;
const HEXBLOOM_LAYER = \(Layer.Hexbloom.index)u;
const CUBE_CORNERS = array<vec3f, 24>(
    \(cubeCornersWGSL()));
"
}

/// assets/shaders/`name`.wgsl, ready to compile: the header, surface.wgsl,
/// then the file. The shaders ship with the game, so a missing one is a
/// broken install.
func loadShader(name: String, encode encode: Bool) -> String {
    shaderHeader(encode) + readShader("surface") + readShader(name)
}

func readShader(name: String) -> String {
    let path = "assets/shaders/\(name).wgsl";
    match readFileString(path) {
        .Ok(text) => text,
        .Err(_) => { fatalError("can't read \(path); run from the game's folder") }
    }
}
