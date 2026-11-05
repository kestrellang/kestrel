// The block shader: chunks of blocks, the sky, shadows, and models (birds
// and cracks). Loaded by renderer.ks after a generated header that defines
// ENCODE_SRGB, MAX_LIGHTS, LIGHT_RADIUS, the glowing blocks' layers and
// CUBE_CORNERS, and after surface.wgsl.
struct Uniforms {
    viewProj: mat4x4f,
    lightViewProj: mat4x4f,
    invViewProj: mat4x4f,
    // xyz: the direction sunlight travels; w: how strong it is, 0 at night.
    sunDir: vec4f,
    // xyz: the eye; w: seconds since start, to drift the clouds.
    camera: vec4f,
    // rgb: the sky at the horizon; w: the view distance, where fog is total.
    fog: vec4f,
    // rgb: sky light from above; w: how much it is night, 0 to 1.
    sky: vec4f,
    // rgb: the sky overhead.
    zenith: vec4f,
    // rgb: the sunlight's color; w: the lairs' gloom, 0 to 1.
    sunColor: vec4f,
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
    lights: array<vec4f, MAX_LIGHTS>,
};
@group(1) @binding(0) var<uniform> chunk: ChunkData;


// Four corners per face, counter-clockwise seen from outside, as offsets
// inside the unit cell. Faces 0-5 are the cube's: east, west, up, down,
// south, north (CUBE_CORNERS, generated from `cubeCorners` in mesh.ks).
// Faces 6-9 are the four sides of a plant's two crossed quads.
const PLANT_CORNERS = array<vec3f, 16>(
    vec3f(0.15, 0, 0.15), vec3f(0.85, 0, 0.85), vec3f(0.85, 1, 0.85), vec3f(0.15, 1, 0.15),
    vec3f(0.85, 0, 0.85), vec3f(0.15, 0, 0.15), vec3f(0.15, 1, 0.15), vec3f(0.85, 1, 0.85),
    vec3f(0.15, 0, 0.85), vec3f(0.85, 0, 0.15), vec3f(0.85, 1, 0.15), vec3f(0.15, 1, 0.85),
    vec3f(0.85, 0, 0.15), vec3f(0.15, 0, 0.85), vec3f(0.15, 1, 0.85), vec3f(0.85, 1, 0.15),
);

fn faceCorner(face: u32, i: u32) -> vec3f {
    if face < 6u {
        return CUBE_CORNERS[face * 4u + i];
    }
    return PLANT_CORNERS[(face - 6u) * 4u + i];
}
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
    let world = chunk.origin.xyz + cell + faceCorner(face, corner);
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
    let world = chunk.origin.xyz + cell + faceCorner(face, QUAD[vertex]);
    return u.lightViewProj * vec4f(world, 1.0);
}

// How much sun reaches `world`, 0 to 1: a 5x5 average of filtered shadow
// map comparisons, for soft, steady edges. Outside the map counts as lit.
fn sunlight(world: vec3f, n: vec3f) -> f32 {
    let light = u.lightViewProj * vec4f(world + n * 0.05, 1.0);
    let uv = vec2f(light.x * 0.5 + 0.5, 0.5 - light.y * 0.5);
    if any(uv < vec2f(0.0)) || any(uv > vec2f(1.0)) || light.z > 1.0 {
        return 1.0;
    }
    let texel = 1.0 / vec2f(textureDimensions(shadowMap));
    var lit = 0.0;
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            lit += textureSampleCompareLevel(shadowMap, shadowSampler, uv + vec2f(f32(x), f32(y)) * texel, light.z);
        }
    }
    return lit / 25.0;
}

// Light from nearby glowing blocks: a soft falloff to zero at
// LIGHT_RADIUS, colored by each light's kind (`Block.glow`): glowcaps' cool
// cyan or the lairs' ember red. It passes through walls; it's cheap and
// reads as a glow.
fn glow(world: vec3f, n: vec3f) -> vec3f {
    var total = vec3f(0.0);
    let count = u32(chunk.origin.w);
    for (var i = 0u; i < count; i++) {
        let to = chunk.lights[i].xyz - world;
        let d = length(to);
        let falloff = clamp(1.0 - d / LIGHT_RADIUS, 0.0, 1.0);
        // Faces turned away get a little, so corners don't go black.
        let facing = select(0.35 + 0.65 * max(dot(n, to / max(d, 0.001)), 0.0), 1.0, d < 0.9);
        let tint = select(vec3f(0.55, 0.95, 0.90), vec3f(1.0, 0.42, 0.16) * 1.3, chunk.lights[i].w > 1.5);
        total += tint * falloff * falloff * facing;
    }
    return min(total, vec3f(1.6));
}

// Hashed value noise, for clouds and stars.
fn hash2(p: vec2f) -> f32 {
    var q = fract(p * vec2f(0.1031, 0.1030));
    q += dot(q, q.yx + 33.33);
    return fract((q.x + q.y) * q.x);
}

fn vnoise(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2f(1.0, 0.0));
    let c = hash2(i + vec2f(0.0, 1.0));
    let d = hash2(i + vec2f(1.0, 1.0));
    return mix(mix(a, b, s.x), mix(c, d, s.x), s.y);
}

fn fbm(start: vec2f) -> f32 {
    var p = start;
    var total = 0.0;
    var amp = 0.5;
    for (var i = 0; i < 5; i++) {
        total += amp * vnoise(p);
        p = p * 2.03 + vec2f(17.1, 9.2);
        amp *= 0.5;
    }
    return total;
}

// The sky's gradient along `dir`: rose-cream at the horizon to blue
// overhead, the glowing cloud sea below, and the sun's halo. Fog
// fades the world into this, so far islands melt into the sky.
fn skyGradient(dir: vec3f) -> vec3f {
    let horizon = u.fog.rgb;
    let zenith = u.zenith.rgb;
    let day = u.sunDir.w;
    let middle = mix(horizon, zenith, 0.5) + vec3f(0.01, -0.01, 0.03) * day;
    let up = clamp(dir.y, 0.0, 1.0);
    var color = mix(horizon, middle, smoothstep(0.0, 0.18, up));
    color = mix(color, zenith, smoothstep(0.18, 0.6, up));
    let sea = horizon * vec3f(1.02, 0.94, 1.04) + vec3f(0.06, 0.06, 0.08) * day;
    color = mix(color, sea, smoothstep(0.0, 0.2, -dir.y));
    let toSun = normalize(-u.sunDir.xyz);
    let s = max(dot(dir, toSun), 0.0);
    let halo = (pow(s, 6.0) * 0.22 + pow(s, 48.0) * 0.4) * smoothstep(-0.3, 0.05, toSun.y);
    return color + u.sunColor.rgb * halo;
}

// A surface's color in the sun, sky light and nearby glow. `facing` is
// how squarely it faces the sun, `ao` its ambient occlusion (0 to 1).
fn lit(world: vec3f, n: vec3f, facing: f32, ao: f32, glowColor: vec3f, rgb: vec3f) -> vec3f {
    let direct = facing * u.sunDir.w * sunlight(world, n);
    // Lavender sky light from above, and a dimmer bounce from the cloud sea.
    let ambient = u.sky.rgb * mix(0.5, 0.7, n.y * 0.5 + 0.5);
    return rgb * (ambient * ao + glowColor * ao + direct * u.sunColor.rgb * mix(0.6, 1.0, ao));
}

// Fog only near the view distance, so chunks fade in at the edge instead
// of popping; everything nearer is clear.
fn fogged(world: vec3f, color: vec3f) -> vec3f {
    let toEye = world - u.camera.xyz;
    let distance = length(toEye);
    let edge = smoothstep(u.fog.w * 0.75, u.fog.w, distance);
    return mix(color, skyGradient(toEye / max(distance, 0.001)), edge);
}

// The lit, fogged color of a texel, keeping its alpha.
fn shade(in: VertexOut, texel: vec4f) -> vec4f {
    let n = NORMALS[in.face];
    let l = normalize(-u.sunDir.xyz);
    // Plants are lit from all sides alike, so their quads don't flicker
    // between light and dark as the sun moves.
    let facing = select(max(dot(n, l), 0.0), 0.6, in.face >= 6u);
    var color = lit(in.world, n, facing, in.light, glow(in.world, n), texel.rgb);
    // Glowing blocks light themselves.
    if in.layer == GLOWCAP_LAYER || in.layer == EMBERCORE_LAYER || in.layer == HEXBLOOM_LAYER {
        color = texel.rgb * 1.15;
    }
    return vec4f(fogged(in.world, color), texel.a);
}

// Models: boxes at any angle (`ModelBox`), for birds and the cracks on a
// block being broken. 36 vertices each: six faces of two triangles.
struct ModelOut {
    @builtin(position) position: vec4f,
    @location(0) world: vec3f,
    @location(1) uv: vec2f,
    @location(2) @interpolate(flat) layer: u32,
    @location(3) normal: vec3f,
};

fn modelCorner(vertex: u32, center: vec3f, ax: vec3f, ay: vec3f, az: vec3f) -> vec3f {
    let local = CUBE_CORNERS[(vertex / 6u) * 4u + QUAD[vertex % 6u]] * 2.0 - 1.0;
    return center + ax * local.x + ay * local.y + az * local.z;
}

@vertex
fn vs_model(@builtin(vertex_index) vertex: u32, @location(0) center: vec4f, @location(1) ax: vec4f, @location(2) ay: vec4f, @location(3) az: vec4f) -> ModelOut {
    let face = vertex / 6u;
    let world = modelCorner(vertex, center.xyz, ax.xyz, ay.xyz, az.xyz);
    let n = NORMALS[face];
    var out: ModelOut;
    out.position = u.viewProj * vec4f(world, 1.0);
    out.world = world;
    out.uv = UVS[QUAD[vertex % 6u]];
    // The top face, the bottom, or a side.
    out.layer = u32(select(select(center.w, ay.w, face == 3u), ax.w, face == 2u));
    out.normal = normalize(ax.xyz * n.x + ay.xyz * n.y + az.xyz * n.z);
    return out;
}

@vertex
fn vs_model_shadow(@builtin(vertex_index) vertex: u32, @location(0) center: vec4f, @location(1) ax: vec4f, @location(2) ay: vec4f, @location(3) az: vec4f) -> @builtin(position) vec4f {
    return u.lightViewProj * vec4f(modelCorner(vertex, center.xyz, ax.xyz, ay.xyz, az.xyz), 1.0);
}

@fragment
fn fs_model(in: ModelOut) -> @location(0) vec4f {
    let texel = textureSample(atlas, atlasSampler, in.uv, in.layer);
    // Clear texels draw nothing; the cracks' pale lips blend over the block.
    if texel.a < 0.05 {
        discard;
    }
    let n = normalize(in.normal);
    let facing = max(dot(n, normalize(-u.sunDir.xyz)), 0.0);
    return toSurface(vec4f(fogged(in.world, lit(in.world, n, facing, 1.0, vec3f(0.0), texel.rgb)), texel.a));
}

// The sky: one triangle covering the screen, behind everything.
struct SkyOut {
    @builtin(position) position: vec4f,
    @location(0) ndc: vec2f,
};

@vertex
fn vs_sky(@builtin(vertex_index) vertex: u32) -> SkyOut {
    let p = vec2f(f32((vertex << 1u) & 2u), f32(vertex & 2u)) * 2.0 - 1.0;
    var out: SkyOut;
    out.position = vec4f(p, 0.0, 1.0);
    out.ndc = p;
    return out;
}

// Clouds over the lairs: ash grey, lit red from below.
fn gloomy(color: vec3f) -> vec3f {
    return mix(color, color * vec3f(0.42, 0.22, 0.26), u.sunColor.w);
}

// Soft clouds: drifting noise on a plane above, warmed on the sun's side, and a sea of puffs below.
fn clouds(dir: vec3f, color: vec3f) -> vec3f {
    let time = u.camera.w;
    let day = u.sunDir.w;
    let toSun = normalize(-u.sunDir.xyz);
    let light = mix(vec3f(0.26, 0.24, 0.44), mix(vec3f(0.98, 0.90, 0.90), u.sunColor.rgb, 0.3), day);
    let bright = mix(vec3f(0.42, 0.34, 0.60), vec3f(1.0, 0.97, 1.0), day);
    if dir.y > 0.0 {
        let p = dir.xz / (dir.y + 0.08) * 1.6 + vec2f(time * 0.012, time * 0.005);
        let n = fbm(p);
        let cover = smoothstep(0.52, 0.76, n) * smoothstep(0.0, 0.25, dir.y);
        // Lighter where the cloud thins toward the sun.
        let toward = fbm(p + toSun.xz * 0.12);
        let lit = clamp(0.55 + (n - toward) * 4.0, 0.0, 1.0);
        let tint = gloomy(mix(light, bright, lit));
        return mix(color, tint, cover * 0.92);
    }
    let p = dir.xz / (-dir.y + 0.04) * 0.7 + vec2f(time * 0.006, time * 0.003);
    let n = fbm(p);
    let puff = smoothstep(0.3, 0.75, n);
    let shadow = mix(vec3f(0.28, 0.20, 0.42), vec3f(0.80, 0.80, 0.92), day);
    let sea = gloomy(mix(shadow, mix(light, bright, 0.6), puff));
    return mix(color, sea, smoothstep(0.0, 0.3, -dir.y) * 0.85);
}

@fragment
fn fs_sky(in: SkyOut) -> @location(0) vec4f {
    let near = u.invViewProj * vec4f(in.ndc, 0.0, 1.0);
    let far = u.invViewProj * vec4f(in.ndc, 1.0, 1.0);
    let dir = normalize(far.xyz / far.w - near.xyz / near.w);
    var color = skyGradient(dir);
    let night = u.sky.w;
    // Stars: a sparse hash over directions, twinkling.
    if dir.y > 0.0 && night > 0.0 {
        let cell = floor(dir * 260.0);
        let h = hash2(cell.xy + vec2f(cell.z * 37.0, cell.z * 11.0));
        let twinkle = 0.6 + 0.4 * sin(u.camera.w * 2.0 + h * 80.0);
        let star = step(0.9965, h) * twinkle * night * smoothstep(0.0, 0.25, dir.y);
        color += vec3f(1.0, 0.88, 1.0) * star;
    }
    // The sun, and the moon opposite it.
    let toSun = normalize(-u.sunDir.xyz);
    let s = dot(dir, toSun);
    color = mix(color, u.sunColor.rgb * 0.3 + vec3f(0.85), smoothstep(0.9988, 0.9993, s));
    color = mix(color, vec3f(0.96, 0.92, 1.0), smoothstep(0.9993, 0.9996, -s) * night);
    color = clouds(dir, color);
    return toSurface(vec4f(color, 1.0));
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    let texel = textureSample(atlas, atlasSampler, in.uv, in.layer);
    // Alpha-tested: leaves and plants have holes.
    if texel.a < 0.3 {
        discard;
    }
    return toSurface(vec4f(shade(in, texel).rgb, 1.0));
}

// Translucent faces, blended over the opaque world by their alpha.
@fragment
fn fs_glass(in: VertexOut) -> @location(0) vec4f {
    return toSurface(shade(in, textureSample(atlas, atlasSampler, in.uv, in.layer)));
}
