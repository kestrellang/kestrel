// test: diagnostics
// stdlib: false

module Test
struct Container {
    let items: ()

    subscript(index: ()) -> () { self.items }
    subscript(index: ()) -> () { self.items } // ERROR: duplicate subscript signature
}
