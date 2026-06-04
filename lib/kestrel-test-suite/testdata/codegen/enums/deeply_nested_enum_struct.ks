// test: execution
// stdlib: true

module Test

struct Inner {
    let value: std.numeric.Int64
}

enum Middle {
    case Value(inner: Inner)
    case Empty
}

struct Outer {
    let middle: Middle
    let extra: std.numeric.Int64
}

func extract(o: Outer) -> std.numeric.Int64 {
    match o.middle {
        .Value(inner: i) => i.value + o.extra,
        .Empty => o.extra
    }
}

@main
func main() -> lang.i64 {
    let outer = Outer(
        middle: Middle.Value(inner: Inner(value: 30)),
        extra: 12
    );
    if extract(outer) != 42 { return 1 }
    0
}
