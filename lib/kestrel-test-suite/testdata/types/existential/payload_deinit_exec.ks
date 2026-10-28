// test: execution
// stdlib: true
// expect-exit: 0
//
// Dropping the last `any P` runs the payload's deinit once. With RcBox it
// does not run while another copy is alive.

module Test

import std.numeric.(Int64)
import std.core.(Cloneable)

var drops: Int64 = 0;

protocol Shape {
    func area() -> Int64
}

struct Res: not Copyable {
    var v: Int64;
    deinit { drops = drops + 1; }
}
extend Res: Shape {
    public func area() -> Int64 { self.v }
}

func inner() -> Int64 {
    let a: any Shape = Res(v: 5);
    let b = a;
    if b.area() != 5 { return 1 }
    if drops != 0 { return 2 }
    0
}

@main
func main() -> Int64 {
    let r = inner();
    if r != 0 { return r }
    if drops != 1 { return 10 + drops }
    0
}
