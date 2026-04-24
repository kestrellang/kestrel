// test: diagnostics
// stdlib: false

module Test
struct Container {
    let items: ()

    subscript(index index: ()) -> () { items }
    subscript(at at: ()) -> () { items }
}
