// test: diagnostics

module Test

func test(foo: (lang.i64) -> (lang.i64) -> lang.i64) {
    let f: (lang.i64) -> (lang.i64) -> lang.i64 = foo;
    let fs: [(lang.i64) -> lang.i64] = [];
}
