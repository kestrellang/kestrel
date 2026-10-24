// test: diagnostics
// stdlib: false

module Test

// Hidden from docs and completion only: lookups and access are unchanged.
@doc(.Hidden)
public struct Hook {
    public var raw: lang.i64
}

@doc(.Hidden)
extend Hook {
    public func read() -> lang.i64 { self.raw }
}

func use(h: Hook) -> lang.i64 { h.read() }
