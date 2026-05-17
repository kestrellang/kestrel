// test: execution
// stdlib: true

module Test

struct Handler {
    let action: (std.numeric.Int64) -> std.numeric.Int64
}

func main() -> lang.i64 {
    let h = Handler(action: { (x) in x * 2 });
    if (h.action)(21) != 42 { return 1 }
    0
}
