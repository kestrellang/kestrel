// test: execution
// stdlib: true
// expect-stdout: 6 true 2\n

// Reading a computed property through the peel calls the target's getter on
// the lent view: `box.area` where only the target declares `area` (a plain
// getter), `box.width` (a `ref` getter), and `xs.first` on an `Array`,
// which reaches `Slice.first` through `Array`'s `Indirection` conformance.
module Test

import std.memory.(OwnedBox)

struct Rect {
    var w: Int64
    var h: Int64
    var area: Int64 { self.w * self.h }
    var width: Int64 { ref { self.w } }
}

@main
func main() {
    let box = OwnedBox(Rect(w: 2, h: 3));
    let sq = OwnedBox(Rect(w: 2, h: 2));
    let xs = [2, 4];
    println("\(box.area) \(sq.width == 2) \(xs.first ?? 0)");
}
