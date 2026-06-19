// test: diagnostics
// stdlib: false

module Main

public func test_panic_in_else(condition: lang.i1) -> lang.i64 {
    if condition {
        42
    } else {
        lang.panic()
    }
}

public func test_panic_in_then(condition: lang.i1) -> lang.i64 {
    if condition {
        lang.panic()
    } else {
        42
    }
}
