// test: execution
// stdlib: true
// expect-exit: 0

// Regression: when a value's type only resolves through an associated-type
// projection (e.g. `String.bytes(unchecked: i)` returns `Int64.BytesYield`,
// which projects to `UInt8`), comparing it against an untyped integer literal
// must default the literal to the value's type, not to `Int64`.
//
// The `==` operator lowers to a Member dispatch where the literal sits in the
// *args*, not the receiver slot. `apply_literal_defaults`'s context-driven
// pass only checked the receiver/callee, so an arg-position literal would
// always default to `Int64` before the deferred Member dispatch could bind
// the parameter type — producing a spurious "expected UInt8 got Int64".

module Test

@main
func main() -> lang.i64 {
    let s = "#hi";
    let firstByte = s.bytes(unchecked: 0);
    if firstByte == 35 { // '#'
        return 0
    }
    1
}
