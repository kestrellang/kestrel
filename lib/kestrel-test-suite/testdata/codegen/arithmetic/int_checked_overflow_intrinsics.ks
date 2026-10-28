// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#160): `*Checked` arithmetic detected overflow by multiplying then
// dividing back, which is fundamentally broken for `minValue` (e.g.
// `Int8(-128).multiplyChecked(-1)` == 128 overflows, but -128*-1 wraps to -128 and
// -128/-1 wraps to -128 == self, so the divide-back check wrongly returned Some).
// The simplified add/sub checks also had sign-edge bugs. Now `addChecked`/
// `subtractChecked`/`multiplyChecked` use overflow-detecting intrinsics
// (lang.iN_{signed,unsigned}_{add,sub,mul}_overflows): LLVM `*.with.overflow`,
// Cranelift a widening check. Covers signed + unsigned, overflow + no-overflow.

module Test

import std.numeric.Int8
import std.numeric.Int64
import std.numeric.UInt8

@main
func main() -> lang.i64 {
    // #160 — the canonical case: -128 * -1 overflows Int8.
    let m: Int8 = -128;
    let n: Int8 = -1;
    match m.multiplyChecked(n) { .Some(v) => return 1, .None => {} }

    // signed multiply no-overflow.
    match (6).multiplyChecked(7) { .Some(v) => { if v != 42 { return 2 } }, .None => return 3 }

    // signed add overflow at the top, and no-overflow.
    match Int64.maxValue.addChecked(1) { .Some(v) => return 4, .None => {} }
    match (5).addChecked(3) { .Some(v) => { if v != 8 { return 5 } }, .None => return 6 }

    // signed subtract overflow at the bottom (minValue - 1).
    match Int64.minValue.subtractChecked(1) { .Some(v) => return 7, .None => {} }

    // unsigned add overflow (255 + 1) and underflow on subtract (0 - 1).
    let u: UInt8 = 255;
    match u.addChecked(1) { .Some(v) => return 8, .None => {} }
    let z: UInt8 = 0;
    match z.subtractChecked(1) { .Some(v) => return 9, .None => {} }

    // unsigned multiply overflow (200 * 2) and no-overflow (10 * 10).
    let a: UInt8 = 200;
    match a.multiplyChecked(2) { .Some(v) => return 10, .None => {} }
    let b: UInt8 = 10;
    match b.multiplyChecked(10) { .Some(v) => { if v != 100 { return 11 } }, .None => return 12 }

    0
}
