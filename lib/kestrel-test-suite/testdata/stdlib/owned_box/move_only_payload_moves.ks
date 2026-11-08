// test: diagnostics
// stdlib: true
//
// An `OwnedBox` of a move-only payload is itself move-only.

module Test

import std.numeric.(Int64)
import std.memory.(OwnedBox)

struct Handle: not Copyable {
    var fd: Int64
}

func run() -> Int64 {
    let a = OwnedBox(Handle(fd: 3));
    let b = a;
    let c = a; // ERROR: use of moved value 'a'
    0
}
