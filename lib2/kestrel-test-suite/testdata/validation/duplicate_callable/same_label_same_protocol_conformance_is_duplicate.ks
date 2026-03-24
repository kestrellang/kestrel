// test: diagnostics
// stdlib: false

module Test
struct Wrapper {
    let value: ()

    init(from value: ()) { self.value = value }
    init(from value: ()) { self.value = () } // ERROR: duplicate initializer signature
}
