// test: diagnostics
// stdlib: true

module Test

struct Point {
    var x: Int64
}

func test() {
    let p = Point(x: 1);
    let a = p?.x; // ERROR: `?.` requires a type that conforms to `OptionalChain`, found `Point`
}
