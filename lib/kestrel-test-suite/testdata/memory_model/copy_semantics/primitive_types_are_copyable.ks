// test: diagnostics
// stdlib: false

module Test

func consume(consuming n: lang.i64) {}

func test() {
    let x = 42;
    consume(x)
}
