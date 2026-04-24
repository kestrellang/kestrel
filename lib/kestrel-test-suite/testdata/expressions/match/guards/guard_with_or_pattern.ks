// test: diagnostics
// stdlib: false

module Main

enum Value {
    case A(n: lang.i64)
    case B(n: lang.i64)
}

func test(v: Value) -> lang.str {
    match v {
        .A(n) or .B(n) if lang.i64_signed_gt(n, 0) => "positive",
        _ => "other"
    }
}
