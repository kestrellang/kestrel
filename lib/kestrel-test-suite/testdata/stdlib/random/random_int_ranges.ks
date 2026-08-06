// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    // Int64.random(in:) closed range stays in bounds, deterministic per seed
    var rng = Lcg64(seed: 42);
    var i: Int64 = 0;
    while i < 200 {
        let roll = Int64.random(in: 1..=6, using: rng);
        if roll < 1 { return 1 }
        if roll > 6 { return 2 }
        i = i + 1
    }

    var a = Lcg64(seed: 5);
    var b = Lcg64(seed: 5);
    if Int64.random(in: 0..<100, using: a) != Int64.random(in: 0..<100, using: b) { return 3 }

    // the caller's generator state advances across calls
    var c = Lcg64(seed: 5);
    let first = Int64.random(in: 0..<1000000, using: c);
    let second = Int64.random(in: 0..<1000000, using: c);
    let third = Int64.random(in: 0..<1000000, using: c);
    if first == second and second == third { return 4 }

    // half-open range excludes the end
    var h = Lcg64(seed: 8);
    var j: Int64 = 0;
    while j < 300 {
        let v = Int64.random(in: 0..<3, using: h);
        if v < 0 { return 5 }
        if v >= 3 { return 6 }
        j = j + 1
    }

    // negative bounds
    var neg = Lcg64(seed: 13);
    var k: Int64 = 0;
    while k < 200 {
        let v = Int64.random(in: -10..=-1, using: neg);
        if v < -10 { return 7 }
        if v > -1 { return 8 }
        k = k + 1
    }

    // full-range draws execute without trapping
    var full = Lcg64(seed: 21);
    let anyI = Int64.random(in: Int64.minValue..=Int64.maxValue, using: full);
    let anyU = UInt64.random(using: full);
    let anyI8 = Int8.random(using: full);
    let anyU16 = UInt16.random(using: full);
    let anySys = Int64.random();

    // narrow types stay in bounds
    var small = Lcg64(seed: 34);
    var m: Int64 = 0;
    while m < 200 {
        let v8 = Int8.random(in: -5..=5, using: small);
        if v8 < -5 { return 9 }
        if v8 > 5 { return 10 }
        let u8 = UInt8.random(in: 0..<10, using: small);
        if u8 >= 10 { return 11 }
        let v16 = Int16.random(in: 100..=200, using: small);
        if v16 < 100 { return 12 }
        if v16 > 200 { return 13 }
        let u32 = UInt32.random(in: 0..=100, using: small);
        if u32 > 100 { return 16 }
        m = m + 1
    }

    // single-value range always returns that value
    var one = Lcg64(seed: 55);
    if Int64.random(in: 4..=4, using: one) != 4 { return 14 }
    if Int64.random(in: 4..<5, using: one) != 4 { return 15 }

    0
}
