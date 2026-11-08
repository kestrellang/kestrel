// test: diagnostics
// stdlib: false
//
// A bare protocol is not a type (existentials design §Syntax, E144). Every
// value position is rejected; bound positions (`T: Shape`, `some Shape`) are
// unchanged.

module Test

protocol Shape {
    func area() -> lang.i64
}

struct Circle { var r: lang.i64; }
extend Circle: Shape {
    public func area() -> lang.i64 { self.r }
}

struct Wrap[T] { var v: T; }

struct Holder {
    var s: Shape; // ERROR: protocol 'Shape' cannot be used as a type; write 'any Shape' for a boxed existential or 'some Shape' for an opaque type
}

func takes(s: Shape) {} // ERROR: protocol 'Shape' cannot be used as a type; write 'any Shape' for a boxed existential or 'some Shape' for an opaque type

func makes() -> Shape { Circle(r: 1) } // ERROR: protocol 'Shape' cannot be used as a type; write 'any Shape' for a boxed existential or 'some Shape' for an opaque type

func wrapped(w: Wrap[Shape]) {} // ERROR: protocol 'Shape' cannot be used as a type; write 'any Shape' for a boxed existential or 'some Shape' for an opaque type

func local() {
    let s: Shape = Circle(r: 1); // ERROR: protocol 'Shape' cannot be used as a type; write 'any Shape' for a boxed existential or 'some Shape' for an opaque type
}

// Bound positions stay legal.
func generic[T](x: T) -> lang.i64 where T: Shape { x.area() }
func opaque(x: some Shape) -> lang.i64 { x.area() }
