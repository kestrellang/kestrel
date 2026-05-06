// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var arr = std.collections.Array[std.numeric.Int64]();
    arr.append(10); arr.append(20); arr.append(30);

    if arr(probe: 0) != 10 { return 1 }
    if arr(probe: 2) != 30 { return 2 }

    let c0 = arr(probeChecked: 0);
    if c0.isNone() { return 3 }
    if c0.unwrap() != 10 { return 4 }

    let cOut = arr(probeChecked: 99);
    if cOut.isSome() { return 5 }

    let s = arr.asSlice();
    if s(probe: 0) != 10 { return 6 }
    if s(probe: 2) != 30 { return 7 }

    0
}
