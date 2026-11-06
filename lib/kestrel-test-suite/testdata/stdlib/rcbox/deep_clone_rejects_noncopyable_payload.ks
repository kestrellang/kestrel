// test: diagnostics
// stdlib: true

// G13 / Bug 2 (unsound accept), on public stdlib surface.
//
// `deepClone` lives on `extend RcBox[T] where T: Copyable`
// (lang/std/memory/rcbox.ks) — deliberately outside the struct body, which is
// relaxed to `T: not Copyable`, because it copies the payload OUT of storage
// with a bitwise read. (`getValue`, its old twin, was removed in favor of the
// `value` place; this test moved to the surviving member.)
//
// It did not. `extension_bounds_hold` skipped any `Copyable`/`Cloneable` where
// clause outright ("copyability is enforced by the move checker / mono"), so
// the bound gated nothing and `RcBox[NC].getValue()` on a `not Copyable`
// payload COMPILED CLEAN and trapped at run time with exit 132 (SIGILL).
// This test replaces that silent miscompile with a compile-time diagnostic.
//
// The shape is "no member", not "does not conform": once the bound is
// enforced, the extension simply does not apply to `RcBox[NC]`, so `deepClone`
// was never a candidate — identical to the `where T: Equatable` control in
// `declarations/extensions/inapplicable_specialized_extension_no_member.ks`.
//
// NOT the same gap as `memory_model/copy_semantics/subscript_read_notcopyable_traps.ks`,
// which covers `Pointer.read()`'s METHOD-level bound and never routes through
// `extension_bounds_hold`.

module Test

import std.numeric.Int64
import std.memory.RcBox

struct NC: not Copyable {
    var v: Int64;
}

func test() -> RcBox[NC] {
    let b = RcBox[NC](NC(v: 5));
    b.deepClone() // ERROR: no member 'deepClone' on type 'RcBox[NC]'
}
