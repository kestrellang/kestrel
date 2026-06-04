// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    var arr = std.collections.Array[std.numeric.Int64]();
    arr.append(10); arr.append(20); arr.append(30);

    // -- Default subscript (index:) read --
    if arr(0) != 10 { return 1 }
    if arr(2) != 30 { return 2 }

    // -- Default subscript (index:) write --
    arr(1) = 25;
    if arr(1) != 25 { return 3 }
    arr(1) = 20;

    // -- Checked subscript read --
    let c0 = arr(checked: 0);
    if c0.isNone() { return 4 }
    if c0.unwrap() != 10 { return 5 }
    let cOut = arr(checked: 99);
    if cOut.isSome() { return 6 }

    // -- Unchecked subscript read/write --
    if arr(unchecked: 0) != 10 { return 7 }
    arr(unchecked: 1) = 99;
    if arr(1) != 99 { return 8 }
    arr(unchecked: 1) = 20;

    // -- Clamped subscript read --
    let cl = arr(clamped: -5);
    if cl.isNone() { return 9 }
    if cl.unwrap() != 10 { return 10 }
    let clHigh = arr(clamped: 100);
    if clHigh.unwrap() != 30 { return 11 }

    // -- Wrapped subscript read --
    let w = arr(wrapped: -1);
    if w.isNone() { return 12 }
    if w.unwrap() != 30 { return 13 }
    let w2 = arr(wrapped: 3);
    if w2.unwrap() != 10 { return 14 }

    // -- Range subscript read --
    let rng = arr(0..<2);
    if rng.count != 2 { return 15 }
    if rng(0) != 10 { return 16 }
    if rng(1) != 20 { return 17 }

    // -- ClosedRange subscript read --
    let cr = arr(0..=2);
    if cr.count != 3 { return 18 }
    if cr(2) != 30 { return 19 }

    // -- Range checked --
    let rcOk = arr(checked: 0..<2);
    if rcOk.isNone() { return 20 }
    let rcBad = arr(checked: 0..<10);
    if rcBad.isSome() { return 21 }

    // -- Range clamped --
    let rcl = arr(clamped: -5..<100);
    if rcl.count != 3 { return 22 }

    // -- ArraySlice subscripts via Slice[T] extension --
    let s = arr.asSlice();
    if s(0) != 10 { return 23 }
    if s(2) != 30 { return 24 }

    let sc = s(checked: 99);
    if sc.isSome() { return 25 }

    let su = s(unchecked: 1);
    if su != 20 { return 26 }

    let scl = s(clamped: -5);
    if scl.isNone() { return 27 }
    if scl.unwrap() != 10 { return 28 }

    let sw = s(wrapped: -1);
    if sw.isNone() { return 29 }
    if sw.unwrap() != 30 { return 30 }

    // -- Range subscript on slice --
    let sr = s(0..<2);
    if sr.count != 2 { return 31 }
    if sr(0) != 10 { return 32 }

    0
}
