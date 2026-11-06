// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Labeled and unlabeled payload sub-patterns bind each payload by position.

module Test

enum E {
    case A(x: Int64, y: Int64)
    case B(Int64)
}

func labeled(e: E) -> Int64 {
    match e {
        .A(x: a, y: b) => a * 10 + b,
        .B(n) => n
    }
}

func unlabeled(e: E) -> Int64 {
    match e {
        .A(a, b) => a * 10 + b,
        .B(n) => n
    }
}

@main
func main() -> lang.i64 {
    if labeled(.A(x: 1, y: 2)) != 12 { return 1 }
    if unlabeled(.A(x: 3, y: 4)) != 34 { return 2 }
    if let .A(x: p, y: _) = E.A(x: 7, y: 8) { if p != 7 { return 3 } } else { return 4 }
    0
}
