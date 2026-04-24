// test: diagnostics
// stdlib: false

module Test

func test() -> lang.i64 {
    let f: () -> lang.i64 = { 1 };
    let g: () -> lang.i64 = { 2 };
    lang.i64_add(f(), g())
}
