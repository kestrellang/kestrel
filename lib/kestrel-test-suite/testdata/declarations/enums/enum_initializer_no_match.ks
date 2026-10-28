// test: diagnostics
// stdlib: true
//
// An enum has no memberwise init: calling it without a matching `init` is
// an error, not a construction.

module Main

import std.numeric.(Int64)

enum Plain {
    case A
}

enum WithInit {
    case A
    case B(Int64)

    init() {
        self = .A;
    }
}

func use() {
    let p = Plain(); // ERROR: no matching overload for 'Plain'
    let w = WithInit(label: 1); // ERROR: no matching overload for 'WithInit'
}
