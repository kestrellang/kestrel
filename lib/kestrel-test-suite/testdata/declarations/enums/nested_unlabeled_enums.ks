// test: diagnostics
// stdlib: false

module Test

enum Inner[T] {
    case Value(T)
    case None
}

enum Outer[T] {
    case Wrapped(Inner[T])
    case Empty
}

func unwrap_nested(o: Outer[lang.i64]) -> lang.i64 {
    match o {
        .Wrapped(inner) => match inner {
            .Value(v) => v,
            .None => 0
        },
        .Empty => 0
    }
}
