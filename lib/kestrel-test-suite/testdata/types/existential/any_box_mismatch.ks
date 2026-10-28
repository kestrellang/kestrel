// test: diagnostics
// stdlib: true
//
// E147: existentials in different boxes are different types; there is no
// implicit or explicit conversion between them (design §4).

module Test

import std.numeric.(Int64)
import std.memory.(OwnedBox)

protocol Shape {
    func area() -> Int64
}

struct Circle { var r: Int64; }
extend Circle: Shape {
    public func area() -> Int64 { 3 * self.r * self.r }
}

struct Square { var side: Int64; }
extend Square: Shape {
    public func area() -> Int64 { self.side * self.side }
}

func takesDefault(s: any Shape) -> Int64 { s.area() }
func takesUnique(s: any[OwnedBox] Shape) -> Int64 { s.area() }

func test() {
    let u: any[OwnedBox] Shape = Circle(r: 1);
    takesDefault(u); // ERROR: cannot convert 'any[OwnedBox] Shape' to 'any Shape': existentials in different boxes are different types
    let d: any Shape = Circle(r: 1);
    takesUnique(d); // ERROR: cannot convert 'any Shape' to 'any[OwnedBox] Shape': existentials in different boxes are different types
}
