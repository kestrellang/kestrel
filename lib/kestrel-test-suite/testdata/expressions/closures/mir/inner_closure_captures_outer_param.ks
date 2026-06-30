// test: diagnostics
// stdlib: false

module Test

func test() -> lang.i64 {
    let f: (lang.i64) -> (lang.i64) -> lang.i64 = { (x) in { (y) in lang.i64_add(x, y) } }; // ERROR(E494)
    let add10 = f(10);
    add10(5)
}
