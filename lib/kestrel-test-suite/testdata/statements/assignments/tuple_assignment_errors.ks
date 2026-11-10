// test: diagnostics
// stdlib: true

// Each target of a tuple assignment is checked like a plain `x = v`. A bare
// `_` is a discard only there: anywhere else it is still E811.
module Test

func test() {
    let a = 1;
    var b = 2;
    (a, b) = (3, 4); // ERROR: cannot assign to immutable variable 'a'
    (b, 5) = (1, 2); // ERROR: cannot assign to this expression
    let t = (_, 1); // ERROR: `_` here isn't a placeholder
}
