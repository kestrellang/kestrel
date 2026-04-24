// test: diagnostics
// stdlib: true
// skip: Array rest patterns with bindings are not yet supported

module Main

func test(arr: [lang.i64]) -> [lang.i64] {
    match arr {
        [_, ..rest] => rest,
        [] => []
    }
}
