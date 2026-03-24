// test: diagnostics
// stdlib: false

module Main

type MyInt = lang.i64;

func test() {
    let x: MyInt = 42;
    let y: lang.i64 = x;
}
