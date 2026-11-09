// test: diagnostics
// stdlib: true

// Only defaulted fields may be left out of a memberwise init, and arguments
// follow declaration order.
module Test

struct S {
    var a: Int64
    var b: Int64 = 3
    var c: Int64
}

func f() {
    let x = S(a: 1); // ERROR(E113): missing argument for field 'c'
    let y = S(a: 1, c: 2, b: 3); // ERROR(E113): no remaining field of struct 'S' takes the 'b' argument
}
