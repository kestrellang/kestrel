// test: execution
// stdlib: true
// expect-exit: 0

module Test

// An initializer with its own type param on a generic type:
// `Holder[Int64](n: 2, extra: "abc")` — `[Int64]` is the type's `T`; the
// init's own `U` is inferred from `extra`.

struct Holder[T] {
    var n: T
    var size: Int64
    init[U](n n: T, extra extra: U) where U: Formattable {
        self.n = n;
        self.size = "\(extra)".bytes.count;
    }
}

@main
func main() -> Int64 {
    let h = Holder[Int64](n: 2, extra: "abc");
    if h.n != 2 { return 1 }
    if h.size != 3 { return 2 }
    let g = Holder(n: "q", extra: 12345);
    if g.size != 5 { return 3 }
    0
}
