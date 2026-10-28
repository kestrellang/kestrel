// test: diagnostics

// `Optional` (and any enum) stores its payload inline, so a struct holding
// `Self?` would be infinitely large. Array storage is heap indirection.

module Test

struct Node {
    var next: Node?; // ERROR: cannot contain itself
}

struct Pair {
    var other: Optional[Pair]; // ERROR: cannot contain itself
}

struct Tree {
    var children: [Tree];
}
