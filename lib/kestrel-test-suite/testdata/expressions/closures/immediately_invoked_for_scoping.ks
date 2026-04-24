// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let result = {
        let a = 10;
        let b = 20;
        lang.i64_add(a, b)
    }();
    result
}
