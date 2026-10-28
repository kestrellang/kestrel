// test: diagnostics
// stdlib: true

// A generic function bounded by `Exitable`, called with `Result[NotExitable, E]`.
// `Result` *declares* `Exitable`, but only conditionally
// (`extend Result[T, E]: Exitable where T: Exitable, E: Formattable`).
// `NotExitable` satisfies neither bound, so the call must be rejected at
// inference with a clean "does not conform" diagnostic — NOT accepted (because
// the conformance is declared) and then ICE at monomorphization on the missing
// `report()` witness. Guards the bound-aware `solve_conforms` path.
module Main
import std.os.Exitable

struct NotExitable { var x: Int64 }

func wantsExitable[T](value: T) where T: Exitable { }

func test() {
    let r: Result[NotExitable, NotExitable] = .Ok(NotExitable(x: 0));
    wantsExitable(r); // ERROR: does not conform to protocol
}
