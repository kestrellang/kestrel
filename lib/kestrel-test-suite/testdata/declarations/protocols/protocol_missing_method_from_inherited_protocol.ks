// test: diagnostics
// stdlib: false
module Test
protocol A { func a() }
protocol B: A { func b() }
struct S: B { // ERROR: does not implement method 'a'
    func b() { }
}
