// test: execution
// stdlib: true
// expect-exit: 0
//
// #188 follow-up: array patterns work on ANY `ArrayMatchable` conformer, not
// just the builtin `Array`/`ArraySlice`. A non-generic user struct whose
// `Element` is a concrete type (no type args) is matched with a fixed-length
// pattern and a prefix+rest pattern. Pins the conformance-driven pipeline:
// inference projects `Trio.Element` (= Int64), and MIR/pattern-matching resolve
// the element type from the conformance binding rather than the first type arg
// (which a non-generic conformer doesn't have).

module Test

import std.core.ArrayMatchable
import std.memory.ArraySlice

struct Trio {
    var a: Int64
    var b: Int64
    var c: Int64
}

extend Trio: ArrayMatchable {
    type Element = Int64
    public func matchLength() -> Int64 { 3 }
    public func matchGet(index: Int64) -> Int64 {
        if index == 0 { self.a } else if index == 1 { self.b } else { self.c }
    }
    public func matchSlice(from: Int64, to: Int64) -> ArraySlice[Int64] {
        ArraySlice(pointer: Pointer(to: self.a).offset(by: from), count: to - from)
    }
}

func full(t: Trio) -> Int64 {
    match t {
        [x, y, z] => x * 100 + y * 10 + z,
        _ => -1
    }
}

func headPlusRest(t: Trio) -> Int64 {
    match t {
        [first, ..rest] => first + rest.count,
        _ => -1
    }
}

@main
func main() -> lang.i32 {
    let t = Trio(a: 1, b: 2, c: 3);
    if full(t) != 123 { return 1 }        // [x,y,z] binds all three
    if headPlusRest(t) != 3 { return 2 }  // first=1, rest=[2,3] count 2 -> 3
    0
}
