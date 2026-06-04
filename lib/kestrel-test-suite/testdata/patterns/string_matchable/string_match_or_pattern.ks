// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.text.String
import std.numeric.Int64

func category(s: String) -> Int64 {
    match s {
        "a" or "b" or "c" => 1,
        "x" or "y" => 2,
        _ => 0
    }
}

@main
func main() -> lang.i64 {
    if category("a") != 1 { return 1 }
    if category("b") != 1 { return 2 }
    if category("c") != 1 { return 3 }
    if category("x") != 2 { return 4 }
    if category("y") != 2 { return 5 }
    if category("z") != 0 { return 6 }
    0
}
