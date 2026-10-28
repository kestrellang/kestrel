// test: diagnostics
// stdlib: true

module Test

func make_adder(n: std.numeric.Int64) -> (std.numeric.Int64) -> std.numeric.Int64 {
    { (x) in x + n } // ERROR(E494)
}

func main() -> lang.i64 {
    let add10 = make_adder(10);
    if add10(32) != 42 { return 1 }
    0
}
