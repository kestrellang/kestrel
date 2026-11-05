module Aerie

/// The block shader. Each instance is one face from `meshChunk`; the vertex
/// shader picks the face's corners from a table by `vertex_index`.
func blockShader() -> String {
"
struct Uniforms {
    viewProj: mat4x4f,
    lightViewProj: mat4x4f,
    // xyz: the direction sunlight travels; w: how strong it is, 0 at night.
    sunDir: vec4f,
    camera: vec4f,
    // rgb: the fog color; w: the view distance, where fog is total.
    fog: vec4f,
    // rgb: sky light from above; w: unused.
    sky: vec4f,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var atlas: texture_2d_array<f32>;
@group(0) @binding(2) var atlasSampler: sampler;
@group(0) @binding(3) var shadowMap: texture_depth_2d;
@group(0) @binding(4) var shadowSampler: sampler_comparison;

// Per chunk: where it sits (xyz) and how many lights reach it (w), then
// those lights' positions.
struct ChunkData {
    origin: vec4f,
    lights: array<vec4f, \(maxLights)>,
};
@group(1) @binding(0) var<uniform> chunk: ChunkData;

const LIGHT_RADIUS = \(lightRadius);
const GLOWCAP_LAYER = \(Layer.Glowcap.index)u;

// Four corners per face, counter-clockwise seen from outside, as offsets
// inside the unit cell. Faces: east, west, up, down, south, north, then
// the four sides of a plant's two crossed quads.
const CORNERS = array<vec3f, 40>(
    \(cubeCornersWGSL())
    vec3f(0.15, 0, 0.15), vec3f(0.85, 0, 0.85), vec3f(0.85, 1, 0.85), vec3f(0.15, 1, 0.15),
    vec3f(0.85, 0, 0.85), vec3f(0.15, 0, 0.15), vec3f(0.15, 1, 0.15), vec3f(0.85, 1, 0.85),
    vec3f(0.15, 0, 0.85), vec3f(0.85, 0, 0.15), vec3f(0.85, 1, 0.15), vec3f(0.15, 1, 0.85),
    vec3f(0.85, 0, 0.15), vec3f(0.15, 0, 0.85), vec3f(0.15, 1, 0.85), vec3f(0.85, 1, 0.15),
);
const NORMALS = array<vec3f, 10>(
    vec3f(1, 0, 0), vec3f(-1, 0, 0), vec3f(0, 1, 0), vec3f(0, -1, 0), vec3f(0, 0, 1), vec3f(0, 0, -1),
    vec3f(0.707, 0, -0.707), vec3f(-0.707, 0, 0.707), vec3f(0.707, 0, 0.707), vec3f(-0.707, 0, -0.707),
);
const UVS = array<vec2f, 4>(vec2f(0, 1), vec2f(1, 1), vec2f(1, 0), vec2f(0, 0));
// Two triangles per quad, split along either diagonal.
const QUAD = array<u32, 12>(0, 1, 2, 0, 2, 3, 1, 2, 3, 1, 3, 0);

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) world: vec3f,
    @location(1) uv: vec2f,
    @location(2) @interpolate(flat) layer: u32,
    @location(3) @interpolate(flat) face: u32,
    @location(4) light: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @location(0) packed: u32, @location(1) shade: u32) -> VertexOut {
    let cell = vec3f(f32(packed & 31u), f32((packed >> 5u) & 31u), f32((packed >> 10u) & 31u));
    let face = (packed >> 15u) & 15u;
    let corner = QUAD[vertex + select(0u, 6u, (shade & 256u) != 0u)];
    let world = chunk.origin.xyz + cell + CORNERS[face * 4u + corner];
    var out: VertexOut;
    out.position = u.viewProj * vec4f(world, 1.0);
    out.world = world;
    out.uv = UVS[corner];
    out.layer = (packed >> 19u) & 255u;
    out.face = face;
    // Ambient occlusion: 0 (a corner boxed in) to 3 (open).
    let ao = f32((shade >> (corner * 2u)) & 3u);
    out.light = 0.45 + 0.55 * ao / 3.0;
    return out;
}

// The shadow pass: only depth, as the sun sees the blocks.
@vertex
fn vs_shadow(@builtin(vertex_index) vertex: u32, @location(0) packed: u32) -> @builtin(position) vec4f {
    let cell = vec3f(f32(packed & 31u), f32((packed >> 5u) & 31u), f32((packed >> 10u) & 31u));
    let face = (packed >> 15u) & 15u;
    let world = chunk.origin.xyz + cell + CORNERS[face * 4u + QUAD[vertex]];
    return u.lightViewProj * vec4f(world, 1.0);
}

// How much sun reaches `world`, 0 to 1: a 3x3 average of shadow map
// comparisons. Outside the shadow map counts as lit.
fn sunlight(world: vec3f, n: vec3f) -> f32 {
    let light = u.lightViewProj * vec4f(world + n * 0.05, 1.0);
    let uv = vec2f(light.x * 0.5 + 0.5, 0.5 - light.y * 0.5);
    if any(uv < vec2f(0.0)) || any(uv > vec2f(1.0)) || light.z > 1.0 {
        return 1.0;
    }
    let texel = 1.0 / vec2f(textureDimensions(shadowMap));
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            lit += textureSampleCompareLevel(shadowMap, shadowSampler, uv + vec2f(f32(x), f32(y)) * texel, light.z);
        }
    }
    return lit / 9.0;
}

// Light from nearby glowing blocks: a soft falloff to zero at
// LIGHT_RADIUS. It passes through walls; it's cheap and reads as a glow.
fn glow(world: vec3f, n: vec3f) -> f32 {
    var total = 0.0;
    let count = u32(chunk.origin.w);
    for (var i = 0u; i < count; i++) {
        let to = chunk.lights[i].xyz - world;
        let d = length(to);
        let falloff = clamp(1.0 - d / LIGHT_RADIUS, 0.0, 1.0);
        // Faces turned away get a little, so corners don't go black.
        let facing = select(0.35 + 0.65 * max(dot(n, to / max(d, 0.001)), 0.0), 1.0, d < 0.9);
        total += falloff * falloff * facing;
    }
    return min(total, 1.4);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    let texel = textureSample(atlas, atlasSampler, in.uv, in.layer);
    // Alpha-tested: leaves and plants have holes. Glass draws as a solid
    // pane for now (translucency comes with milestone 6).
    if texel.a < 0.3 {
        discard;
    }
    let n = NORMALS[in.face];
    let l = normalize(-u.sunDir.xyz);
    // Plants are lit from all sides alike, so their quads don't flicker
    // between light and dark as the sun moves.
    let facing = select(max(dot(n, l), 0.0), 0.6, in.face >= 6u);
    let direct = facing * u.sunDir.w * sunlight(in.world, n);
    // Sky light from above, and a dimmer bounce from the cloud sea below.
    let ambient = u.sky.rgb * mix(0.45, 0.65, n.y * 0.5 + 0.5);
    let glowColor = vec3f(0.45, 0.95, 0.85) * glow(in.world, n);
    var color = texel.rgb * (ambient * in.light + glowColor * in.light + direct * vec3f(1.0, 0.90, 0.76) * 1.0 * mix(0.6, 1.0, in.light));
    if in.layer == GLOWCAP_LAYER {
        color = texel.rgb * 1.1;
    }
    let distance = length(u.camera.xyz - in.world);
    // A light haze everywhere, closing in fully at the view distance so
    // chunks fade in instead of popping.
    let haze = 1.0 - exp(-distance * 0.002);
    let edge = smoothstep(u.fog.w * 0.55, u.fog.w, distance);
    color = mix(color, u.fog.rgb, max(haze, edge));
    return vec4f(color, 1.0);
}
"
}

/// The 2D overlay: screen-space rectangles in pixels, each either a solid
/// color or a tinted atlas tile (for hotbar icons).
let overlayShader: String = "
struct Screen {
    size: vec4f,
};
@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var atlas: texture_2d_array<f32>;
@group(0) @binding(2) var atlasSampler: sampler;

const CORNERS = array<vec2f, 6>(vec2f(0, 0), vec2f(1, 0), vec2f(1, 1), vec2f(0, 0), vec2f(1, 1), vec2f(0, 1));

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
    @location(2) @interpolate(flat) layer: i32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @location(0) rect: vec4f, @location(1) color: vec4f, @location(2) tile: vec4f) -> VertexOut {
    let corner = CORNERS[vertex];
    let pixel = rect.xy + corner * rect.zw;
    var out: VertexOut;
    out.position = vec4f(pixel.x / screen.size.x * 2.0 - 1.0, 1.0 - pixel.y / screen.size.y * 2.0, 0.0, 1.0);
    out.uv = corner;
    out.color = color;
    out.layer = i32(tile.x);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    if in.layer < 0 {
        return in.color;
    }
    let texel = textureSampleLevel(atlas, atlasSampler, in.uv, in.layer, 0.0);
    return vec4f(texel.rgb * in.color.rgb, texel.a * in.color.a);
}
";
