// test: execution
// stdlib: true
// expect-exit: 0

// Regression test for the mono-dependent double-read bug (was KNOWN BROKEN).
//
// A local whose type is a bare type parameter has MONO-DEPENDENT copy
// behavior: pre-mono the compiler cannot know whether `T` is Copyable.
// Reading such a local twice is legal source — `T` may well be Copyable, so
// the frontend does not (and must not) reject it.
//
// Lowering used to get it wrong. `lower_expr_inner`'s `HirExpr::Local` arm
// treated mono-dependent exactly like definitely-non-Copyable and TOOK the
// value out of the slot on every consuming read, marking it uninit. Taking is
// only valid for a LAST use. The second read then hit a vacated slot:
//   - debug compiler: `debug_assert!` "consuming read of an already-moved var"
//   - two reads in ONE block: OSSA verify "address ValueId(N) is uninit"
//   - two reads in DIFFERENT blocks: slipped through, because the OSSA linear
//     ownership check was block-local (audit finding F34, since fixed)
//
// The take rule came from #141 (`Optional.take()`/`replace()`, where a clone
// would bitwise-alias storage the following `self = .None` drops) and was
// correct for that reassign shape. Commit a30332b2's #107 "let-via-address"
// then made every plain `let` bind as an address slot, which put ordinary
// generic locals on the same path — including stdlib `Slice.first(where:)`
// (`predicate(elem)` then `.Some(elem)`), so a DEBUG compiler could not build
// any program at all, hello world included. That stdlib shape was a separate
// argument-lowering bug, fixed in 3662a94e; it is still exercised below.
//
// The fix here: the mono-dependent arm takes only when `is_single_use` says
// the local has exactly one read in the whole HIR arena — a conservative
// stand-in for "last use" that branches and loops cannot defeat. A multi-read
// local copies, which is what the language says a Copyable-by-default `T`
// does. The #141 take/replace shapes read `self` once and still take.
//
// Two narrower attempts were tried and rejected before this one: restricting
// the take to inout-borrow slots regressed
// `drop_elab/forin_cloneable_single_clone` (an extra clone per iteration), and
// falling back to a copy when the slot is already vacated produces IR the OSSA
// verifier rejects. A third — making `copy_is_mono_dependent` answer `false`
// for bare type params — fixes this test and breaks 7 others with
// double-frees; see the doc comment in `kestrel-mir/src/ty_query.rs`.

module Test

import std.numeric.Int64
import std.text.String
import std.collections.Array
import std.result.Optional

// `elem` is read twice. Legal: `T` may be Copyable.
func readTwice[T](value: T) -> T? {
    let elem = value;
    let firstRead = elem;
    let secondRead = elem;
    .Some(secondRead)
}

@main
func main() -> lang.i32 {
    if let hit = readTwice(7) {
        if hit != 7 { return 10 }
    } else {
        return 20
    }

    // The stdlib shape that makes this a build-stopper for debug compilers.
    let words = ["alpha", "beta", "gamma"];
    if let found = words.first(where: { it == "beta" }) {
        if found != "beta" { return 30 }
    } else {
        return 40
    }

    0
}
