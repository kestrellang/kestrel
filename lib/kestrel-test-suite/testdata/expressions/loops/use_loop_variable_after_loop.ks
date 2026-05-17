// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    while true {
        let counter: lang.i64 = 0;
        break;
    }
    counter // ERROR: undefined
}
