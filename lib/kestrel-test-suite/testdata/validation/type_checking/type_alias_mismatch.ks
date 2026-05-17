// test: diagnostics
// stdlib: false

module Main

type MyInt = lang.i64;

func test() {
    let x: MyInt = "not an lang.i64"; // ERROR
}
