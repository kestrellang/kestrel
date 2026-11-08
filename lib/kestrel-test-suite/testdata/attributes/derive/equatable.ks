// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// `@derive(Equatable)`: a struct compares every field in declaration order;
// an enum compares the case, then every payload value. `==` and `!=` come
// from `Equatable`'s own protocol extension.

module Main

import std.numeric.(Int64)

@derive(Equatable)
struct Point {
    var x: Int64
    var y: Int64
}

@derive(Equatable)
struct Named {
    var name: String
    var at: Point
}

@derive(Equatable)
struct Unit { }

@derive(Equatable)
enum Shape {
    case Circle(radius: Int64)
    case Rect(Int64, Int64)
    case Empty
}

@derive(Equatable)
enum Single {
    case Only(Int64)
}

@main
func main() -> Int64 {
    if Point(x: 1, y: 2) != Point(x: 1, y: 2) { return 1 }
    if Point(x: 1, y: 2) == Point(x: 1, y: 3) { return 2 }
    if Point(x: 0, y: 2) == Point(x: 1, y: 2) { return 3 }
    let a = Named(name: "a", at: Point(x: 1, y: 1));
    if a != Named(name: "a", at: Point(x: 1, y: 1)) { return 4 }
    if a == Named(name: "b", at: Point(x: 1, y: 1)) { return 5 }
    if Unit() != Unit() { return 6 }
    if Shape.Circle(radius: 1) != Shape.Circle(radius: 1) { return 7 }
    if Shape.Circle(radius: 1) == Shape.Circle(radius: 2) { return 8 }
    if Shape.Rect(1, 2) == Shape.Rect(2, 1) { return 9 }
    if Shape.Rect(1, 2) != Shape.Rect(1, 2) { return 10 }
    if Shape.Empty != Shape.Empty { return 11 }
    if Shape.Empty == Shape.Circle(radius: 0) { return 12 }
    if Single.Only(3) != Single.Only(3) { return 13 }
    if Single.Only(3) == Single.Only(4) { return 14 }
    0
}
