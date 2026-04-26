// test: diagnostics
// stdlib: false

module Test
struct Widget {
    let x: ()

    init(value: ()) { x = value }
    init(value: (), extra: ()) { x = value }
}
