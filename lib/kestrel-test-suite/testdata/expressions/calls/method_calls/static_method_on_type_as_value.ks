// test: execution
// stdlib: true
// expect-exit: 0

// The legitimate half of the `Type.method` rule: a *static* method has no
// receiver, so naming it through its type is a complete function value.
// Guards the rejection of `Type.instanceMethod` (see
// instance_method_on_type_as_value.ks) from over-reaching, including the
// extension-declared case that reaches name resolution by a different path.

module Test

struct Box {
    let v: Int64
    static func stat(x: Int64) -> Int64 { x + 300 }
}

extend Box {
    static func extStat(x: Int64) -> Int64 { x + 1 }
}

func apply(f: (Int64) -> Int64, a: Int64) -> Int64 { f(a) }

@main
func main() -> lang.i32 {
    if apply(Box.stat, 7) != 307 { return 1 }
    if apply(Box.extStat, 7) != 8 { return 2 }
    // Still callable directly.
    if Box.stat(7) != 307 { return 3 }
    0
}
