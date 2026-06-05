// test: execution
// stdlib: true

module Test

struct Handler {
    let f: (std.numeric.Int64) -> std.numeric.Int64
}

func triple(x: std.numeric.Int64) -> std.numeric.Int64 {
    x * 3
}

@main
func main() -> lang.i64 {
    let h = Handler(f: triple);
    if (h.f)(14) != 42 { return 1 }
    0
}
