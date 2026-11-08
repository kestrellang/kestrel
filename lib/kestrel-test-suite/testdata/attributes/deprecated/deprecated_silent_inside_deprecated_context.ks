// test: diagnostics
// stdlib: false

module Test

@deprecated
func old() -> lang.i64 { 1 }

@deprecated
struct OldBox {
    var value: lang.i64

    // Nested in a deprecated type: no warning.
    func fetch() -> lang.i64 { old() }
}

// An extension of a deprecated type is part of it.
extend OldBox {
    func twice() -> OldBox { OldBox(value: old()) }
}

// A deprecated function may use deprecated things.
@deprecated
func older(b: OldBox) -> lang.i64 { b.value }

// The declarations themselves never warn; only uses do.
func fresh() -> lang.i64 { 0 }
