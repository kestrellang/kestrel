// test: diagnostics
// stdlib: false
module Test
protocol A { func a() }
protocol B: A { func b() }
struct S: A, B {
    func a() { }
    func b() { }
}
