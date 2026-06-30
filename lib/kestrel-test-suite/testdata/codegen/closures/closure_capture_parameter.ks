// test: diagnostics
// stdlib: true

module Test

func make_multiplier(factor: std.numeric.Int64) -> (std.numeric.Int64) -> std.numeric.Int64 {
    { (x) in x * factor } // ERROR(E494)
}

func main() -> lang.i64 {
    let times3 = make_multiplier(3);
    if times3(14) != 42 { return 1 }
    0
}
