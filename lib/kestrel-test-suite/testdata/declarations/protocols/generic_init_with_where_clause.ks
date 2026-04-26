// test: diagnostics
// stdlib: false
module Test
public protocol Iterator {
    type Item
}

public protocol Collectable {
    type Item

    init[I](from iter: I) where I: Iterator, I.Item = Item
}
