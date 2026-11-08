// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// An enum's initializers are called like a struct's: `Mode()` and
// `Mode(level)` resolve to the `init` declared in the enum or an extension.
// An enum has no memberwise init, so a call that matches none is an error
// (enum_initializer_no_match.ks).

module Main

import std.numeric.(Int64)

enum Mode {
    case Fast
    case Slow(Int64)

    init() {
        self = .Fast;
    }
}

extend Mode {
    init(level: Int64) {
        self = .Slow(level);
    }

    func level() -> Int64 {
        match self {
            .Fast => 0,
            .Slow(n) => n,
        }
    }
}

@main
func main() -> Int64 {
    if Mode().level() != 0 { return 1 }
    if Mode(7).level() != 7 { return 2 }
    0
}
