// test: execution
// stdlib: true

module Test

struct Handler {
    let f: (std.num.Int64) -> std.num.Int64
}

func triple(x: std.num.Int64) -> std.num.Int64 {
    x * 3
}

func main() -> lang.i64 {
    let h = Handler(f: triple);
    if (h.f)(14) != 42 { return 1 }
    0
}
