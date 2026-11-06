// test: diagnostics
// stdlib: true
//
// `indirect` on an enum with no recursive case boxes nothing: a warning,
// and the enum keeps its ordinary (here bitwise Copyable) semantics.

module Test

import std.numeric.(Int64)

indirect enum Shape { // WARNING: `indirect` has no effect: enum 'Shape' has no recursive case
    case Circle(Int64)
    case Square(Int64)
}

func copies() -> Int64 {
    let a = Shape.Circle(1);
    let b = a;
    let c = a;
    match b {
        .Circle(r) => r,
        .Square(s) => s
    }
}
