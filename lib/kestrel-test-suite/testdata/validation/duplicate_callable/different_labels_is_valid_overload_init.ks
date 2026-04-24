// test: diagnostics
// stdlib: false

module Test
struct Point {
    let x: ()

    init(value value: ()) { x = value }
    init(from from: ()) { x = from }
}
