// test: diagnostics

module Test

func test(foo: lang.i64, bar: lang.i64) -> lang.i64 {
    [foo, bar];
    (foo, bar);
    (foo)
}
