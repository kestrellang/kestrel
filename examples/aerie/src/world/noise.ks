module Aerie

/// 2D gradient noise (Perlin's improved noise, in two dimensions) from a
/// seeded permutation table, plus fractal sums of it. Output is roughly
/// -1 to 1. Everything is deterministic in the seed, so a world can be
/// regenerated instead of saved.
struct Noise {
    /// 512 entries: a shuffle of 0..<256, twice, so lookups can add
    /// without wrapping.
    var perm: [Int64]

    init(seed: Int64) {
        var p: [Int64] = [];
        for i in 0..<256 {
            p.append(i);
        }
        // Fisher-Yates with a 31-bit LCG; products stay under 2^62.
        var state = seed % 2147483648;
        if state < 0 { state = state + 2147483648; }
        for step in 0..<255 {
            let i = 255 - step;
            state = (state * 1103515245 + 12345) % 2147483648;
            let j = state % (i + 1);
            let t = p(i);
            p(i) = p(j);
            p(j) = t;
        }
        var doubled = p;
        for v in p {
            doubled.append(v);
        }
        self.perm = doubled;
    }

    /// Noise at (x, y): smooth, about -1 to 1, one feature per unit.
    func sample(x: Float32, y: Float32) -> Float32 {
        let fx = x.floor();
        let fy = y.floor();
        let xi = (fx.toInt64() ?? 0) & 255;
        let yi = (fy.toInt64() ?? 0) & 255;
        let dx = x - fx;
        let dy = y - fy;
        let u = fade(dx);
        let v = fade(dy);
        let aa = self.perm(self.perm(xi) + yi);
        let ab = self.perm(self.perm(xi) + yi + 1);
        let ba = self.perm(self.perm(xi + 1) + yi);
        let bb = self.perm(self.perm(xi + 1) + yi + 1);
        let bottom = lerp(grad(aa, dx, dy), grad(ba, dx - 1.0, dy), u);
        let top = lerp(grad(ab, dx, dy - 1.0), grad(bb, dx - 1.0, dy - 1.0), u);
        lerp(bottom, top, v) * 1.4
    }

    /// `octaves` layers of noise, each at twice the frequency and half the
    /// weight of the last, normalized back to about -1 to 1.
    func fractal(x: Float32, y: Float32, octaves octaves: Int64) -> Float32 {
        var total: Float32 = 0.0;
        var weight: Float32 = 1.0;
        var scale: Float32 = 1.0;
        var norm: Float32 = 0.0;
        for _ in 0..<octaves {
            total = total + self.sample(x * scale, y * scale) * weight;
            norm = norm + weight;
            weight = weight * 0.5;
            scale = scale * 2.0;
        }
        total / norm
    }
}

/// Perlin's ease curve, 6t^5 - 15t^4 + 10t^3.
func fade(t: Float32) -> Float32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

func lerp(a: Float32, b: Float32, t: Float32) -> Float32 {
    a + (b - a) * t
}

/// The dot product of (x, y) with one of eight gradient directions.
func grad(hash: Int64, x: Float32, y: Float32) -> Float32 {
    match hash & 7 {
        0 => x + y,
        1 => x - y,
        2 => -x + y,
        3 => -x - y,
        4 => x,
        5 => -x,
        6 => y,
        _ => -y
    }
}
