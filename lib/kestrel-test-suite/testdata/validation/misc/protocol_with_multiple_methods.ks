// test: diagnostics
// stdlib: false

module Test
protocol Hashable {
    func hash() -> lang.i64
    func isEqual(to other to: lang.i64) -> lang.i1
}
