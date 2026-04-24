// test: diagnostics
// stdlib: false

module Test

enum Pair[A, B] {
    case Value(A, B)
    case Empty
}

func first(p: Pair[lang.i64, lang.str]) -> lang.i64 {
    match p {
        .Value(a, _b) => a,
        .Empty => 0
    }
}
