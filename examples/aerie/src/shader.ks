module Aerie

/// The block shader. Each instance is one face from `meshChunk`; the vertex
/// shader picks the face's corners from a table by `vertex_index`.
func blockShader() -> String {
"
struct Uniforms {
    viewProj: mat4x4f,
    sunDir: vec4f,
    camera: vec4f,
    // rgb: the fog color; w: the view distance, where fog is total.
    fog: vec4f,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
// Where the chunk being drawn sits in the world.
@group(1) @binding(0) var<uniform> chunkOrigin: vec4f;
@group(0) @binding(1) var atlas: texture_2d_array<f32>;
@group(0) @binding(2) var atlasSampler: sampler;

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
    let world = chunkOrigin.xyz + cell + CORNERS[face * 4u + corner];
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
    let direct = select(max(dot(n, l), 0.0), 0.6, in.face >= 6u);
    // A cool sky from above, warm bounce light from the cloud sea below.
    let ambient = mix(vec3f(0.30, 0.25, 0.30), vec3f(0.38, 0.44, 0.58), n.y * 0.5 + 0.5);
    var color = texel.rgb * (ambient * in.light + direct * vec3f(1.0, 0.90, 0.76) * 0.85 * mix(0.6, 1.0, in.light));
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
