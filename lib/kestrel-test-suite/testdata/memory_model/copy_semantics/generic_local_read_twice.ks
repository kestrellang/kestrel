// test: execution
// stdlib: true
// expect-exit: 0

// KNOWN BROKEN — documents an open bug, see the analysis below.
//
// A local whose type is a bare type parameter has MONO-DEPENDENT copy
// behavior: pre-mono the compiler cannot know whether `T` is Copyable.
// Reading such a local twice is legal source — `T` may well be Copyable, so
// the frontend does not (and must not) reject it.
//
// Lowering gets it wrong. `lower_expr_inner`'s `HirExpr::Local` arm treats
// mono-dependent exactly like definitely-non-Copyable and TAKES the value out
// of the slot on a consuming read, marking it uninit. Taking is only valid for
// a LAST use, which single-pass lowering cannot know. The second read then
// hits a vacated slot:
//   - debug compiler: `debug_assert!` "consuming read of an already-moved var"
//   - two reads in ONE block: OSSA verify "address ValueId(N) is uninit"
//   - two reads in DIFFERENT blocks: slips through, because the OSSA linear
//     ownership check is block-local (audit finding F34)
//
// The take rule came from #141 (`Optional.take()`/`replace()`, where a clone
// would bitwise-alias storage the following `self = .None` drops) and was
// correct for that reassign shape. Commit a30332b2's #107 "let-via-address"
// then made every plain `let` bind as an address slot, which put ordinary
// generic locals on the same path — including stdlib `Slice.first(where:)`
// (`predicate(elem)` then `.Some(elem)`), so a DEBUG compiler cannot build any
// program at all, hello world included.
//
// A correct fix needs last-use (liveness) information that MIR lowering does
// not currently have; the move-vs-clone decision itself belongs in
// `kestrel-copy-fold`. Two narrower attempts were tried and rejected:
// restricting the take to inout-borrow slots regressed
// `drop_elab/forin_cloneable_single_clone` (an extra clone per iteration), and
// falling back to a copy when the slot is already vacated produces IR the OSSA
// verifier rejects.

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
