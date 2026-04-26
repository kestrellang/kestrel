// test: diagnostics
// stdlib: false

module Main

enum Inner {
    case A(x: lang.i64)
    case B
}

enum Outer {
    case Wrap(inner: Inner)
    case Empty
}

func extract(o: Outer) -> lang.i64 {
    match o {
        .Wrap(.A(x)) => x,
        .Wrap(.B) => 0,
        .Empty => 0
    }
}
