// test: diagnostics
// stdlib: true

module Test

func make_multiplier(factor: std.num.Int64) -> (std.num.Int64) -> std.num.Int64 {
    { (x) in x * factor } // ERROR: cannot return a closure that captures variables
}

func main() -> lang.i64 {
    let times3 = make_multiplier(3);
    if times3(14) != 42 { return 1 }
    0
}
