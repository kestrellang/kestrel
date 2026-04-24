// test: diagnostics
// stdlib: false

module Test
func increment(mutating n: lang.i64) {
    n = lang.i64_add(n, 1);
}
func test() {
    let x = 5;
    increment(x) // ERROR: mutating
}
