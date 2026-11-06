// test: diagnostics
// stdlib: true
//
// `unsafe` on an initializer, a computed property and a subscript: every
// use needs an unsafe context, the property's and subscript's accessors
// included.

module Test

struct Cell {
    var n: Int64

    unsafe init(start start: Int64) {
        self.n = start;
    }

    unsafe var doubled: Int64 { self.n + self.n }

    unsafe subscript(i: Int64) -> Int64 { self.n }
}

func f() -> Int64 {
    let c = Cell(start: 1); // ERROR: 'Cell.init' is unsafe and requires an `unsafe` expression or block
    let a = c.doubled; // ERROR: 'doubled' is unsafe and requires an `unsafe` expression or block
    c(0) // ERROR: 'Cell.subscript' is unsafe and requires an `unsafe` expression or block
}

func g() -> Int64 {
    let c = unsafe Cell(start: 1);
    let a = unsafe c.doubled;
    unsafe c(0)
}
