// test: diagnostics
// stdlib: false

module Test
struct Container {
    let items: ()

    subscript(index: ()) -> () { items }
    subscript(index: ()) -> () { items } // ERROR: duplicate subscript signature
}
