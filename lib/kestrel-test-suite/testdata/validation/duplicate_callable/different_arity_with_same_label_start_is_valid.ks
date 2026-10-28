// test: diagnostics
// stdlib: false

module Test
struct Widget {
    let x: ()

    init(value: ()) { self.x = value }
    init(value: (), extra: ()) { self.x = value }
}
