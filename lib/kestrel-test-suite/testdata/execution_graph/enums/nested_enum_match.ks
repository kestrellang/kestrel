// test: diagnostics
// stdlib: false

module Main

enum Inner {
    case A(x: lang.i64)
    case B(y: lang.i64)
}

enum Outer {
    case Left(inner: Inner)
    case Right(inner: Inner)
}

func getValue(outer: Outer) -> lang.i64 {
    match outer {
        .Left(inner) => match inner {
            .A(x) => x,
            .B(y) => y
        },
        .Right(inner) => match inner {
            .A(x) => lang.i64_mul(x, 2),
            .B(y) => lang.i64_mul(y, 2)
        }
    }
}
