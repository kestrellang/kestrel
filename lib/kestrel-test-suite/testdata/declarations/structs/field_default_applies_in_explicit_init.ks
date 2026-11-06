// test: execution
// stdlib: true
// backends: cranelift,llvm

// A stored field's `= expr` default is accepted as initializing the field
// when an explicit `init` doesn't assign it (the initializer analyzer skips
// defaulted fields), so the init must store the default: every init stores
// field defaults before its body runs.
module Test

import std.numeric.(Int64)

struct S {
    var a: Int64
    var b: Int64 = 5

    init(x: Int64) { self.a = x; }
}

@main
func main() -> lang.i64 {
    let s = S(1);
    if s.b != 5 { return 1; }
    0
}
