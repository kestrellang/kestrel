// test: diagnostics
// stdlib: false

module Main

func getInt() -> lang.i64 { 42 }

func test() -> lang.i64 {
    let x = getInt();
    x
}
