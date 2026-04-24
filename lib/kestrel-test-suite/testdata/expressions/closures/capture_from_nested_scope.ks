// test: diagnostics
// stdlib: false

module Main

func test() -> () -> lang.i64 {
    let outer = 100;
    if true {
        let inner = 10;
        { lang.i64_add(outer, inner) }
    } else {
        { outer }
    }
}
