// test: diagnostics
// stdlib: false

module Test
struct Point {
    let x: ()

    init(value value: ()) { self.x = value }
    init(from from: ()) { self.x = from }
}
