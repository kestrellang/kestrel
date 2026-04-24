// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let outer: lang.i64 = 10;
    if true {
        lang.i64_add(outer, 5)
    } else {
        lang.i64_sub(outer, 5)
    }
}
