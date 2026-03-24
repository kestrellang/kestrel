// test: diagnostics
// stdlib: true

module Test

func compose(
    f: (std.num.Int64) -> std.num.Int64,
    g: (std.num.Int64) -> std.num.Int64
) -> (std.num.Int64) -> std.num.Int64 {
    { (x) in g(f(x)) }
}

func main() -> lang.i64 {
    let add10 = { (x: std.num.Int64) in x + 10 };
    let double = { (x: std.num.Int64) in x * 2 };
    let composed = compose(add10, double);
    // (11 + 10) * 2 = 42
    if composed(11) != 42 { return 1 }
    0
}
