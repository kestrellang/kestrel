// test: diagnostics
// stdlib: false

module Test
struct Container {
    let items: ()

    subscript(index index: ()) -> () { self.items }
    subscript(at at: ()) -> () { self.items }
}
