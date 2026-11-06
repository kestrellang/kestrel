// Every fragment shader's output goes through `toSurface`. Shading is in
// linear color, and an sRGB surface encodes it on write; a browser's
// canvas offers no sRGB format, so there ENCODE_SRGB is true and the
// shader applies the sRGB curve itself. Without it, the browser build
// comes out too dark and too contrasty.
fn toSurface(c: vec4f) -> vec4f {
    if !ENCODE_SRGB {
        return c;
    }
    let rgb = clamp(c.rgb, vec3f(0.0), vec3f(1.0));
    let low = rgb * 12.92;
    let high = 1.055 * pow(rgb, vec3f(1.0 / 2.4)) - 0.055;
    return vec4f(select(high, low, rgb <= vec3f(0.0031308)), c.a);
}
