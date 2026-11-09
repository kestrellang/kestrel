// test: diagnostics
// stdlib: true

module Main

// The binding's ref-decay default used to report progress forever when the
// subscript's element type was an error (the index didn't resolve), so
// inference never finished. It must stop with just the name error.

func f(xs: Array[Int64]) {
    let e = xs(nope); // ERROR: undefined name 'nope'
}
