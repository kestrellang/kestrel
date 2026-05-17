// test: diagnostics
// stdlib: false

module Test
protocol Hashable {
    func hash() -> lang.i64
    func equals(other to: lang.i64) -> lang.i1
}
