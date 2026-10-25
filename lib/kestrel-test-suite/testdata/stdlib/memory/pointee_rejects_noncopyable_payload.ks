// test: diagnostics
// stdlib: true

// G13 / Bug 2, twin of `stdlib/rcbox/get_value_rejects_noncopyable_payload.ks`
// on the other piece of affected public stdlib surface.
//
// `pointee` lives on `extend Pointer[T] where T: Copyable`
// (lang/std/memory/pointer.ks) because its getter is a raw `lang.ptr_read` —
// a bitwise duplication of the pointee, which is exactly what a `not Copyable`
// type forbids. `extension_bounds_hold` used to skip `Copyable` where clauses
// entirely, so selecting `Pointer[NC].pointee` COMPILED CLEAN and trapped at
// run time with exit 132 (SIGILL). Now the extension does not apply and the
// member is not a candidate.
//
// Distinct from the neighbouring `pointer_cast_non_copyable.ks` (casting, not
// reading) and from
// `memory_model/copy_semantics/subscript_read_notcopyable_traps.ks`, whose
// `Pointer.read()` carries a METHOD-level bound that never routes through
// `extension_bounds_hold` and is tracked separately.

module Test

import std.numeric.Int64
import std.memory.Pointer

struct NC: not Copyable {
    var v: Int64;
}

func test() -> Int64 {
    let p = Pointer[NC].nullPointer();
    p.pointee.v // ERROR: no member 'pointee' on type 'Pointer[NC]'
}
