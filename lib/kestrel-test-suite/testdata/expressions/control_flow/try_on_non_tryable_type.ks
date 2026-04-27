// test: diagnostics
// stdlib: false

module Main

func getValue() -> lang.i64 {
    42
}

func test() {
    let x = try getValue(); // ERROR: Tryable
}
