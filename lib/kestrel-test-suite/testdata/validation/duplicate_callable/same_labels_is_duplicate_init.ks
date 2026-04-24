// test: diagnostics
// stdlib: false

module Test
struct Point {
    let x: ()

    init(value: ()) { x = value }
    init(value: ()) { x = () } // ERROR: duplicate initializer signature
}
