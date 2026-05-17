// test: diagnostics
// stdlib: false

module Main

struct Node {
    let next: Node // ERROR: cannot contain itself
}
