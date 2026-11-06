// The 2D overlay: screen-space rectangles in pixels, each a solid color,
// a tinted block tile, or a tinted layer of the interface atlas (sprites,
// block icons and font glyphs), smooth or, for pixel art, crisp. Loaded
// after a generated header (ENCODE_SRGB) and surface.wgsl.
struct Screen {
    size: vec4f,
};
@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var atlas: texture_2d_array<f32>;
@group(0) @binding(2) var atlasSampler: sampler;
@group(0) @binding(3) var ui: texture_2d_array<f32>;
@group(0) @binding(4) var uiSampler: sampler;

const CORNERS = array<vec2f, 6>(vec2f(0, 0), vec2f(1, 0), vec2f(1, 1), vec2f(0, 0), vec2f(1, 1), vec2f(0, 1));

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
    @location(2) @interpolate(flat) layer: i32,
    @location(3) @interpolate(flat) source: i32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @location(0) rect: vec4f, @location(1) color: vec4f, @location(2) tile: vec4f) -> VertexOut {
    let corner = CORNERS[vertex];
    let pixel = rect.xy + corner * rect.zw;
    var out: VertexOut;
    out.position = vec4f(pixel.x / screen.size.x * 2.0 - 1.0, 1.0 - pixel.y / screen.size.y * 2.0, 0.0, 1.0);
    out.uv = vec2f(mix(tile.z, tile.w, corner.x), corner.y);
    out.color = color;
    out.layer = i32(tile.x);
    out.source = i32(tile.y);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    // Sample before any branch: `textureSample` needs uniform control flow
    // for its mip level, and a plain fill just ignores the result.
    let layer = max(in.layer, 0);
    let block = textureSampleLevel(atlas, atlasSampler, in.uv, layer, 0.0);
    let sprite = textureSample(ui, uiSampler, in.uv, layer);
    // Pixel art and the pixel font (source 2): the middle of the 24-pixel
    // grid cell under this fragment, unfiltered, so the pixels stay crisp
    // at any size.
    let snapped = (floor(in.uv * 24.0) + 0.5) / 24.0;
    let crisp = textureSampleLevel(ui, uiSampler, snapped, layer, 0.0);
    let texel = select(select(block, sprite, in.source == 1), crisp, in.source == 2);
    if in.layer < 0 {
        return toSurface(in.color);
    }
    return toSurface(vec4f(texel.rgb * in.color.rgb, texel.a * in.color.a));
}
