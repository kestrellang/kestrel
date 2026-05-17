// test: diagnostics
// stdlib: false

module Main

enum E {
    case A(x: lang.i64, y: lang.i64)
}

func test(e: E) -> lang.i64 {
    match e {
        .A(x: 1, y: _) => 1,
        .A(x: _, y: 1) => 2,
        .A(x: 1, y: 1) => 3, // WARN: unreachable
        .A(x: _, y: _) => 4
    }
}
