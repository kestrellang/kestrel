// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// `@derive(Defaultable)` on an enum builds its one `@default` case, with
// each payload value defaulted. `Mode()` calls the derived `init()`.

module Main

import std.numeric.(Int64)

@derive(Equatable, Defaultable)
enum Mode {
    case Fast
    @default
    case Slow(level: Int64, label: String)
}

@derive(Equatable, Defaultable)
enum Toggle {
    @default
    case Off
    case On
}

@main
func main() -> Int64 {
    if Mode() != Mode.Slow(level: 0, label: "") { return 1 }
    if Toggle() != Toggle.Off { return 2 }
    0
}
