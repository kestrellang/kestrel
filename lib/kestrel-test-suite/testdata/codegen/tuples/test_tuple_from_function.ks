// test: execution
// stdlib: true

module Test

func make_pair(a: std.numeric.Int64, b: std.numeric.Int64) -> (std.numeric.Int64, std.numeric.Int64) {
    (a, b)
}

func main() -> lang.i64 {
    let t = make_pair(20, 22);
    if t.0 + t.1 != 42 { return 1 }
    0
}
