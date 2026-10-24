// test: execution
// stdlib: true
// expect-exit: 0
//
// Two instantiations of one generic type conform separately; each gets its
// own table (design edge cases).

module Test

import std.numeric.(Int64)
import std.core.(String)

protocol Sized2 {
    func weight() -> Int64
}

struct Box[T] { var v: T; }
extend Box[Int64]: Sized2 {
    public func weight() -> Int64 { self.v }
}
extend Box[String]: Sized2 {
    public func weight() -> Int64 { self.v.byteCount }
}

@main
func main() -> Int64 {
    let xs: Array[any Sized2] = [Box(v: 7), Box(v: "abc")];
    var total: Int64 = 0;
    for x in xs { total = total + x.weight(); }
    if total != 10 { return 1 }
    0
}
