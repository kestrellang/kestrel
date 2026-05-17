// test: diagnostics
// stdlib: false

module Main

type MyInt = lang.i64;
type YourInt = MyInt;

func test() {
    let x: YourInt = 42;
    let y: lang.i64 = x;
}
