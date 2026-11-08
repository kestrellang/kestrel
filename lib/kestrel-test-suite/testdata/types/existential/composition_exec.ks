// test: execution
// stdlib: true
// expect-exit: 0
//
// `any P and Q`: call both sides, and upcast to each component
// (design §5, composition).

module Test

import std.numeric.(Int64)

protocol Drawable {
    func draw() -> Int64
}

protocol Printable {
    func describe() -> Int64
}

struct Widget {}
extend Widget: Drawable {
    public func draw() -> Int64 { 10 }
}
extend Widget: Printable {
    public func describe() -> Int64 { 20 }
}

func drawIt(d: any Drawable) -> Int64 { d.draw() }
func printIt(p: any Printable) -> Int64 { p.describe() }

@main
func main() -> Int64 {
    let w: any Drawable and Printable = Widget();
    if w.draw() != 10 { return 1 }
    if w.describe() != 20 { return 2 }
    if drawIt(w) != 10 { return 3 }
    if printIt(w) != 20 { return 4 }
    // Bound order doesn't matter.
    let v: any Printable and Drawable = w;
    if v.draw() != 10 { return 5 }
    0
}
