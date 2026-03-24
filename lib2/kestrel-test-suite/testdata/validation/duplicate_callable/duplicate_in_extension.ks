// test: diagnostics
// stdlib: false

module Test
struct Point { let x: () }

extend Point {
    func move(x: ()) { }
    func move(x: ()) { } // ERROR: duplicate function signature
}
