// test: diagnostics
// stdlib: false

module Test
struct Point {
    let x: ()

    init(value: ()) { self.x = value }
    init(value: ()) { self.x = () } // ERROR: duplicate initializer signature
}
