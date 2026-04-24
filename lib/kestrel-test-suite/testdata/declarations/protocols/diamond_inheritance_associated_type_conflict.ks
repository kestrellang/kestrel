// test: diagnostics
// stdlib: false
module Test
protocol A {
    type Element
}

protocol B {
    type Element
}

protocol C: A, B { // ERROR: conflicting associated type 'Element'
}
