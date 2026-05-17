// test: diagnostics
// stdlib: false

module Main

enum Expr {
    case Add(left: lang.i64, right: lang.i64)
    case Sub(left: lang.i64, right: lang.i64)
    case Mul(left: lang.i64, right: lang.i64)
}

func test(e: Expr) -> lang.i64 {
    match e {
        .Add(left, right) or .Sub(left, right) => lang.i64_add(left, right),
        .Mul(left, right) => lang.i64_mul(left, right)
    }
}
