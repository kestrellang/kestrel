// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    // nextUInt64(below:) stays in range and is deterministic per seed
    var rng = Lcg64(seed: 42);
    var i: Int64 = 0;
    while i < 200 {
        let v = rng.nextUInt64(below: 6);
        if v >= 6 { return 1 }
        i = i + 1
    }

    // zero and one bounds return 0 without drawing conditions
    if rng.nextUInt64(below: 0) != 0 { return 2 }
    if rng.nextUInt64(below: 1) != 0 { return 3 }

    // same seed reproduces the same stream
    var a = Lcg64(seed: 7);
    var b = Lcg64(seed: 7);
    if a.nextUInt64(below: 1000) != b.nextUInt64(below: 1000) { return 4 }
    if a.nextUInt64(below: 1000) != b.nextUInt64(below: 1000) { return 5 }

    // nextFloat64 in [0, 1)
    var f = Lcg64(seed: 9);
    var j: Int64 = 0;
    while j < 100 {
        let x = f.nextFloat64();
        if x < 0.0 { return 6 }
        if x >= 1.0 { return 7 }
        j = j + 1
    }

    // nextFloat32 in [0, 1)
    var f32rng = Lcg64(seed: 10);
    var k: Int64 = 0;
    while k < 100 {
        let x = f32rng.nextFloat32();
        if x < 0.0 { return 8 }
        if x >= 1.0 { return 9 }
        k = k + 1
    }

    // nextBool produces both values over a long stream
    var brng = Lcg64(seed: 11);
    var trues: Int64 = 0;
    var n: Int64 = 0;
    while n < 200 {
        if brng.nextBool() { trues = trues + 1 }
        n = n + 1
    }
    if trues == 0 { return 10 }
    if trues == 200 { return 11 }

    0
}
