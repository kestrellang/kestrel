// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// `@derive(Formattable)` prints `Name(label: value, ...)`; an enum prints its
// case, positionally for unlabeled payloads, and a payload-less case (or a
// field-less struct) prints just its name. Width and padding apply to the
// whole value. Interpolation uses it.
//
// Fields are formatted with default options, so a `String` field prints
// unquoted (the design's `debug: true` needs stdlib support first).

module Main

import std.numeric.(Int64)

@derive(Formattable)
struct Version {
    var major: Int64
    var minor: Int64
}

@derive(Formattable)
struct User {
    var name: String
    var version: Version
}

@derive(Formattable)
struct Marker { }

@derive(Formattable)
enum Shape {
    case Circle(radius: Int64)
    case Rect(Int64, Int64)
    case Empty
}

@main
func main() -> Int64 {
    let v = Version(major: 1, minor: 2);
    if "\(v)" != "Version(major: 1, minor: 2)" { return 1 }
    if "\(User(name: "Ada", version: v))" != "User(name: Ada, version: Version(major: 1, minor: 2))" { return 2 }
    if "\(Marker())" != "Marker" { return 3 }
    if "\(Shape.Circle(radius: 5))" != "Circle(radius: 5)" { return 4 }
    if "\(Shape.Rect(1, 2))" != "Rect(1, 2)" { return 5 }
    if "\(Shape.Empty)" != "Empty" { return 6 }
    if "\(Shape.Empty:>7)" != "  Empty" { return 7 }
    if "\(v:<26)|" != "Version(major: 1, minor: 2)|" { return 8 }
    if "\(Shape.Rect(1, 2):^12)" != " Rect(1, 2) " { return 9 }
    0
}
