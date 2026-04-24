// test: diagnostics
// stdlib: false

module Test

enum Triple[A, B, C] {
    case Value(A, B, C)
    case Empty
}

func get_middle(t: Triple[lang.i64, lang.str, lang.i1]) -> lang.str {
    match t {
        .Value(_a, b, _c) => b,
        .Empty => ""
    }
}
