// test: diagnostics
// stdlib: true

module Test

func apply(f: (Int64) -> Int64) -> Int64 { f(1) }

func test() {
    let y = _; // ERROR(E811): `_` here isn't a placeholder
    // closure literals reset the boundary: use `it`
    let z = apply({ _ + 1 }); // ERROR(E811): `_` here isn't a placeholder
}
