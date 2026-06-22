// Exact decimal digit generation for floating-point formatting.
// Hand-written (NOT generated) — single source of truth shared by Float32/Float64.
//
// The IEEE-754 value is decomposed exactly into `m * 2^e` (m an integer
// significand, e a binary exponent) and all decimal digits are produced with
// exact big-integer arithmetic, so rounding is round-to-nearest-even of the
// *stored* binary value — never the float-arithmetic double-rounding that the
// old `value*10^n / round / trunc` pipeline suffered (issues #161, #216).
//
// The big unsigned integer is a little-endian `Array[UInt32]` of base-2^32
// limbs with no trailing zero limbs (the empty array is zero). Only the
// operations the digit pipeline needs are implemented; division is restricted
// to small divisors plus the `5^a * 2^b` denominators that decimal scaling
// produces, so no general long division is required.

module std.numeric

import std.numeric.Int64
import std.numeric.UInt8
import std.numeric.UInt32
import std.numeric.UInt64
import std.collections.Array
import std.text.String

// ---------------------------------------------------------------------------
// big-unsigned-integer primitives
// ---------------------------------------------------------------------------

fileprivate func bnTrim(n: Array[UInt32]) -> Array[UInt32] {
    var hi = n.count;
    while hi > 0 and n(hi - 1) == UInt32.zero { hi = hi - 1 };
    var out = Array[UInt32]();
    var i: Int64 = 0;
    while i < hi { out.append(n(i)); i = i + 1 };
    out
}

fileprivate func bnIsZero(n: Array[UInt32]) -> Bool { n.count == 0 }

fileprivate func bnFromU64(v: UInt64) -> Array[UInt32] {
    let mask = UInt64(from: 4294967295);
    var out = Array[UInt32]();
    out.append(UInt32(from: v.bitwiseAnd(mask)));
    out.append(UInt32(from: v.shiftRight(by: 32)));
    bnTrim(out)
}

fileprivate func bnMulSmall(n: Array[UInt32], m: UInt32) -> Array[UInt32] {
    let mask = UInt64(from: 4294967295);
    let mm = UInt64(from: m);
    var out = Array[UInt32]();
    var carry = UInt64.zero;
    var i: Int64 = 0;
    while i < n.count {
        let prod = UInt64(from: n(i)).multiply(mm).add(carry);
        out.append(UInt32(from: prod.bitwiseAnd(mask)));
        carry = prod.shiftRight(by: 32);
        i = i + 1
    };
    while carry > UInt64.zero {
        out.append(UInt32(from: carry.bitwiseAnd(mask)));
        carry = carry.shiftRight(by: 32)
    };
    bnTrim(out)
}

fileprivate func bnShlBits(n: Array[UInt32], k: Int64) -> Array[UInt32] {
    if bnIsZero(n) { return n };
    let limbShift = k / 32;
    let bitShift = k % 32;
    var out = Array[UInt32]();
    var z: Int64 = 0;
    while z < limbShift { out.append(UInt32.zero); z = z + 1 };
    if bitShift == 0 {
        var i: Int64 = 0;
        while i < n.count { out.append(n(i)); i = i + 1 }
    } else {
        var carry = UInt32.zero;
        var i: Int64 = 0;
        while i < n.count {
            let cur = n(i);
            out.append(cur.shiftLeft(by: bitShift).bitwiseOr(carry));
            carry = cur.shiftRight(by: 32 - bitShift);
            i = i + 1
        };
        if carry != UInt32.zero { out.append(carry) }
    };
    bnTrim(out)
}

fileprivate func bnShrBits(n: Array[UInt32], k: Int64) -> Array[UInt32] {
    let limbShift = k / 32;
    let bitShift = k % 32;
    if limbShift >= n.count { return Array[UInt32]() };
    var out = Array[UInt32]();
    var i = limbShift;
    if bitShift == 0 {
        while i < n.count { out.append(n(i)); i = i + 1 }
    } else {
        while i < n.count {
            let lo = n(i).shiftRight(by: bitShift);
            var hi = UInt32.zero;
            if i + 1 < n.count { hi = n(i + 1).shiftLeft(by: 32 - bitShift) };
            out.append(lo.bitwiseOr(hi));
            i = i + 1
        }
    };
    bnTrim(out)
}

fileprivate func bnTestBit(n: Array[UInt32], idx: Int64) -> Bool {
    if idx < 0 { return false };
    let limb = idx / 32;
    if limb >= n.count { return false };
    n(limb).shiftRight(by: idx % 32).bitwiseAnd(UInt32.one) == UInt32.one
}

fileprivate func bnAnyBitBelow(n: Array[UInt32], idx: Int64) -> Bool {
    if idx <= 0 { return false };
    let fullLimbs = idx / 32;
    let remBits = idx % 32;
    var i: Int64 = 0;
    while i < fullLimbs and i < n.count {
        if n(i) != UInt32.zero { return true };
        i = i + 1
    };
    if remBits > 0 and fullLimbs < n.count {
        let mask = UInt32.one.shiftLeft(by: remBits).subtract(UInt32.one);
        if n(fullLimbs).bitwiseAnd(mask) != UInt32.zero { return true }
    };
    false
}

fileprivate func bnIncrement(n: Array[UInt32]) -> Array[UInt32] {
    var out = Array[UInt32]();
    var i: Int64 = 0;
    while i < n.count { out.append(n(i)); i = i + 1 };
    let allOnes = UInt32(from: 4294967295);
    var carry = true;
    var j: Int64 = 0;
    while j < out.count and carry {
        if out(j) == allOnes {
            out(j) = UInt32.zero
        } else {
            out(j) = out(j).add(UInt32.one);
            carry = false
        };
        j = j + 1
    };
    if carry { out.append(UInt32.one) };
    bnTrim(out)
}

fileprivate func bnCmp(a: Array[UInt32], b: Array[UInt32]) -> Int64 {
    if a.count != b.count {
        if a.count < b.count { return -1 };
        return 1
    };
    var i = a.count - 1;
    while i >= 0 {
        if a(i) != b(i) {
            if a(i) < b(i) { return -1 };
            return 1
        };
        i = i - 1
    };
    0
}

// a - b, assuming a >= b
fileprivate func bnSub(a: Array[UInt32], b: Array[UInt32]) -> Array[UInt32] {
    let mask = UInt64(from: 4294967295);
    let base = UInt64(from: 4294967296);
    var out = Array[UInt32]();
    var borrow: Int64 = 0;
    var i: Int64 = 0;
    while i < a.count {
        var av = UInt64(from: a(i));
        let bv = if i < b.count { UInt64(from: b(i)) } else { UInt64.zero };
        let sub = bv.add(UInt64(from: borrow));
        if av < sub {
            av = av.add(base);
            borrow = 1
        } else {
            borrow = 0
        };
        out.append(UInt32(from: av.subtract(sub).bitwiseAnd(mask)));
        i = i + 1
    };
    bnTrim(out)
}

fileprivate func bnMul(a: Array[UInt32], b: Array[UInt32]) -> Array[UInt32] {
    if bnIsZero(a) or bnIsZero(b) { return Array[UInt32]() };
    let mask = UInt64(from: 4294967295);
    var acc = Array[UInt64]();
    var z: Int64 = 0;
    while z < a.count + b.count { acc.append(UInt64.zero); z = z + 1 };
    var i: Int64 = 0;
    while i < a.count {
        var carry = UInt64.zero;
        let av = UInt64(from: a(i));
        var j: Int64 = 0;
        while j < b.count {
            let cur = acc(i + j).add(av.multiply(UInt64(from: b(j)))).add(carry);
            acc(i + j) = cur.bitwiseAnd(mask);
            carry = cur.shiftRight(by: 32);
            j = j + 1
        };
        acc(i + b.count) = acc(i + b.count).add(carry);
        i = i + 1
    };
    var out = Array[UInt32]();
    var k: Int64 = 0;
    while k < acc.count { out.append(UInt32(from: acc(k).bitwiseAnd(mask))); k = k + 1 };
    bnTrim(out)
}

// floor(n / d) for a small (<= 32-bit) divisor
fileprivate func bnDivSmall(n: Array[UInt32], d: UInt64) -> Array[UInt32] {
    var rem = UInt64.zero;
    var qRev = Array[UInt32]();
    var i = n.count - 1;
    while i >= 0 {
        let cur = rem.shiftLeft(by: 32).bitwiseOr(UInt64(from: n(i)));
        qRev.append(UInt32(from: cur.divide(d)));
        rem = cur.modulo(d);
        i = i - 1
    };
    var q = Array[UInt32]();
    var j = qRev.count - 1;
    while j >= 0 { q.append(qRev(j)); j = j - 1 };
    bnTrim(q)
}

fileprivate func bnPow5(k: Int64) -> Array[UInt32] {
    var out = Array[UInt32]();
    out.append(UInt32.one);
    var i: Int64 = 0;
    while i < k { out = bnMulSmall(out, UInt32(from: 5)); i = i + 1 };
    out
}

// round(num / (5^da * 2^db)) with round-half-to-even, using only small-divisor
// division plus one multiply (floor(num/den) via nested floors, then exact
// remainder for the tie test).
fileprivate func bnDivRound5_2(num: Array[UInt32], da: Int64, db: Int64) -> Array[UInt32] {
    var q = bnShrBits(num, db);
    var a: Int64 = 0;
    while a < da { q = bnDivSmall(q, UInt64(from: 5)); a = a + 1 };
    let den = bnShlBits(bnPow5(da), db);
    let r = bnSub(num, bnMul(q, den));
    let twoR = bnShlBits(r, 1);
    let c = bnCmp(twoR, den);
    if c > 0 { return bnIncrement(q) };
    if c == 0 and bnTestBit(q, 0) { return bnIncrement(q) };
    q
}

// decimal digits of a big integer, most-significant first ([0] for zero)
fileprivate func bnToDecimalDigits(n0: Array[UInt32]) -> Array[Int64] {
    var out = Array[Int64]();
    if bnIsZero(n0) { out.append(0); return out };
    var n = n0;
    var digitsRev = Array[Int64]();
    let ten = UInt64(from: 10);
    while bnIsZero(n) == false {
        var rem = UInt64.zero;
        var qRev = Array[UInt32]();
        var i = n.count - 1;
        while i >= 0 {
            let cur = rem.shiftLeft(by: 32).bitwiseOr(UInt64(from: n(i)));
            qRev.append(UInt32(from: cur.divide(ten)));
            rem = cur.modulo(ten);
            i = i - 1
        };
        digitsRev.append(Int64(from: rem));
        var q = Array[UInt32]();
        var j = qRev.count - 1;
        while j >= 0 { q.append(qRev(j)); j = j - 1 };
        n = bnTrim(q)
    };
    var k = digitsRev.count - 1;
    while k >= 0 { out.append(digitsRev(k)); k = k - 1 };
    out
}

fileprivate func bitLen64(v: UInt64) -> Int64 {
    var n = v;
    var c: Int64 = 0;
    while n > UInt64.zero { c = c + 1; n = n.shiftRight(by: 1) };
    c
}

// ---------------------------------------------------------------------------
// public-to-module digit-generation entry points (operate on m * 2^e)
// ---------------------------------------------------------------------------

/// The `sig` correctly-rounded leading decimal digits of `m * 2^e` (> 0), plus
/// the base-10 exponent of the leading digit: value ≈ d0.d1…d_{sig-1} × 10^decExp.
struct FloatSig { var digits: Array[Int64]; var decExp: Int64 }

/// Round(value × 10^precision) as a digit array (most-significant first), where
/// value = m × 2^e. The caller places the decimal point `precision` digits from
/// the right. Exact: round-half-to-even of the stored value.
func floatFixedDigits(m: UInt64, e: Int64, precision: Int64) -> Array[Int64] {
    if m == UInt64.zero {
        var z = Array[Int64]();
        z.append(0);
        return z
    };
    // value × 10^p = m × 5^p × 2^(e+p)
    var num = bnFromU64(m);
    var p: Int64 = 0;
    while p < precision { num = bnMulSmall(num, UInt32(from: 5)); p = p + 1 };
    let twoExp = e + precision;
    if twoExp >= 0 {
        return bnToDecimalDigits(bnShlBits(num, twoExp))
    };
    bnToDecimalDigits(bnDivRound5_2(num, 0, 0 - twoExp))
}

/// `sig` correctly-rounded significant digits of `m × 2^e` (> 0) with the
/// decimal exponent of the leading digit. Used for scientific notation and as
/// the basis for shortest-round-trip output.
func floatSigDigits(m: UInt64, e: Int64, sig: Int64) -> FloatSig {
    if m == UInt64.zero {
        var zeros = Array[Int64]();
        var i: Int64 = 0;
        while i < sig { zeros.append(0); i = i + 1 };
        return FloatSig(digits: zeros, decExp: 0)
    };
    let approx = Float64(from: e + bitLen64(m) - 1)
        .multiply(Float64(floatLiteral: 0.30102999566398114));
    var decExp = Int64(raw: lang.cast_f64_i64(approx.floor().raw));

    var digits = Array[Int64]();
    var tries: Int64 = 0;
    while tries < 4 {
        let q = sig - 1 - decExp;          // N = round(value × 10^q) has `sig` digits
        var num = bnFromU64(m);
        var da: Int64 = 0;
        var db: Int64 = 0;
        if q >= 0 { num = bnMul(num, bnPow5(q)) } else { da = 0 - q };
        let twoExp = e + q;
        if twoExp >= 0 { num = bnShlBits(num, twoExp) } else { db = 0 - twoExp };
        digits = bnToDecimalDigits(bnDivRound5_2(num, da, db));
        if digits.count == sig {
            return FloatSig(digits: digits, decExp: decExp)
        };
        // The estimate is at most ~1 off; the correction is monotone so this
        // converges in 1–2 iterations. `tries` bounds it defensively.
        decExp = decExp + (digits.count - sig);
        tries = tries + 1
    };
    FloatSig(digits: digits, decExp: decExp)
}

// Compare X = numX * 2^e2X * 5^e5X  vs  Y = numY * 2^e2Y * 5^e5Y, where the
// exponents may be negative (denominators). Returns -1/0/1. Clears negative
// exponents by scaling both sides equally, then compares as big integers.
fileprivate func bnScaledCmp(
    numX: Array[UInt32], e2X: Int64, e5X: Int64,
    numY: Array[UInt32], e2Y: Int64, e5Y: Int64
) -> Int64 {
    var a2x = e2X;
    var a2y = e2Y;
    let m2 = if a2x < a2y { a2x } else { a2y };
    if m2 < 0 { a2x = a2x - m2; a2y = a2y - m2 };
    var a5x = e5X;
    var a5y = e5Y;
    let m5 = if a5x < a5y { a5x } else { a5y };
    if m5 < 0 { a5x = a5x - m5; a5y = a5y - m5 };
    let x = bnShlBits(bnMul(numX, bnPow5(a5x)), a2x);
    let y = bnShlBits(bnMul(numY, bnPow5(a5y)), a2y);
    bnCmp(x, y)
}

/// Shortest decimal digit sequence that round-trips back to `m * 2^e` (> 0),
/// with the base-10 exponent of the leading digit. `lowerGapIsHalf` must be
/// true exactly when the value sits at the lower edge of its binade with a
/// half-ulp gap below (normal, fraction == 0, and not the smallest normal) —
/// the caller computes it from the raw bits.
///
/// Strategy: ask `floatSigDigits` for the correctly-rounded `sig`-digit value
/// for sig = 1, 2, … and return the first whose decimal lands inside the
/// rounding interval `(L, U)` of `m*2^e` — Dragon4 boundary handling, with the
/// interval closed iff `m` is even (round-to-nearest-even ties).
func floatShortestDigits(m: UInt64, e: Int64, lowerGapIsHalf: Bool) -> FloatSig {
    if m == UInt64.zero {
        var z = Array[Int64]();
        z.append(0);
        return FloatSig(digits: z, decExp: 0)
    };
    let mEven = m.bitwiseAnd(UInt64.one) == UInt64.zero;
    // Upper midpoint  U = (2m+1) * 2^(e-1).
    let uNum = bnFromU64(m.multiply(UInt64(from: 2)).add(UInt64.one));
    let uE2 = e - 1;
    // Lower midpoint  L = (2m-1) * 2^(e-1)  [uniform]  or  (4m-1) * 2^(e-2) [half].
    var lNum = bnFromU64(m.multiply(UInt64(from: 2)).subtract(UInt64.one));
    var lE2 = e - 1;
    if lowerGapIsHalf {
        lNum = bnFromU64(m.multiply(UInt64(from: 4)).subtract(UInt64.one));
        lE2 = e - 2
    };

    var sig: Int64 = 1;
    var best = floatSigDigits(m, e, 17);   // 17 sig digits always round-trips
    while sig <= 17 {
        let sr = floatSigDigits(m, e, sig);
        // candidate value = Dint * 10^f = Dint * 2^f * 5^f
        var dv = UInt64.zero;
        var i: Int64 = 0;
        while i < sr.digits.count {
            dv = dv.multiply(UInt64(from: 10)).add(UInt64(from: sr.digits(i)));
            i = i + 1
        };
        let dint = bnFromU64(dv);
        let f = sr.decExp - (sr.digits.count - 1);
        let cmpHigh = bnScaledCmp(dint, f, f, uNum, uE2, 0);
        let cmpLow = bnScaledCmp(dint, f, f, lNum, lE2, 0);
        let aboveLow = cmpLow > 0 or (cmpLow == 0 and mEven);
        let belowHigh = cmpHigh < 0 or (cmpHigh == 0 and mEven);
        if aboveLow and belowHigh {
            return sr
        };
        sig = sig + 1
    };
    best
}

// ---------------------------------------------------------------------------
// numeric-string assembly (no sign / trim / pad — the caller applies those)
// ---------------------------------------------------------------------------

/// Fixed-point string of `m × 2^e` (>= 0) rounded to `precision` fractional
/// digits, e.g. "123.46", "0.000", "2".
func floatFixedString(m: UInt64, e: Int64, precision: Int64) -> String {
    let digits = floatFixedDigits(m, e, precision);
    let len = digits.count;
    var out = String();
    if precision == 0 {
        var i: Int64 = 0;
        while i < len { out.appendByte(UInt8(from: digits(i) + 48)); i = i + 1 };
        return out
    };
    if len <= precision {
        out.appendByte(UInt8(from: 48));   // '0'
        out.appendByte(UInt8(from: 46));   // '.'
        var pad = precision - len;
        var c: Int64 = 0;
        while c < pad { out.appendByte(UInt8(from: 48)); c = c + 1 };
        var i: Int64 = 0;
        while i < len { out.appendByte(UInt8(from: digits(i) + 48)); i = i + 1 }
    } else {
        let intLen = len - precision;
        var i: Int64 = 0;
        while i < intLen { out.appendByte(UInt8(from: digits(i) + 48)); i = i + 1 };
        out.appendByte(UInt8(from: 46));   // '.'
        while i < len { out.appendByte(UInt8(from: digits(i) + 48)); i = i + 1 }
    };
    out
}

// Append a base-10 exponent ("e18", "e-324") to `out`. `upper` selects 'E'.
fileprivate func appendExponent(out: String, decExp: Int64, upper: Bool) -> String {
    var s = out;
    if upper { s.appendByte(UInt8(from: 69)) } else { s.appendByte(UInt8(from: 101)) };
    var ex = decExp;
    if ex < 0 { s.appendByte(UInt8(from: 45)); ex = 0 - ex };
    if ex == 0 {
        s.appendByte(UInt8(from: 48))
    } else {
        var tmp = String();
        while ex > 0 { tmp.appendByte(UInt8(from: ex % 10 + 48)); ex = ex / 10 };
        var k = tmp.byteCount - 1;
        while k >= 0 { s.appendByte(tmp.bytes(unchecked: k)); k = k - 1 }
    };
    s
}

/// Fixed-point rendering of a digit sequence whose leading digit is at 10^decExp
/// (used for shortest output, so there is no fixed precision). Examples:
/// digits=[3,…],decExp=-1 → "0.30000000000000004"; [1],decExp=2 → "100".
func floatShortestFixedString(sr: FloatSig) -> String {
    let digits = sr.digits;
    let len = digits.count;
    let decExp = sr.decExp;
    var out = String();
    if decExp < 0 {
        out.appendByte(UInt8(from: 48));   // "0"
        out.appendByte(UInt8(from: 46));   // "."
        var z: Int64 = 0;
        while z < (0 - decExp) - 1 { out.appendByte(UInt8(from: 48)); z = z + 1 };
        var i: Int64 = 0;
        while i < len { out.appendByte(UInt8(from: digits(i) + 48)); i = i + 1 }
    } else if decExp >= len - 1 {
        // integer, possibly with trailing zeros
        var i: Int64 = 0;
        while i < len { out.appendByte(UInt8(from: digits(i) + 48)); i = i + 1 };
        var z: Int64 = 0;
        while z < decExp - (len - 1) { out.appendByte(UInt8(from: 48)); z = z + 1 }
    } else {
        let intLen = decExp + 1;
        var i: Int64 = 0;
        while i < intLen { out.appendByte(UInt8(from: digits(i) + 48)); i = i + 1 };
        out.appendByte(UInt8(from: 46));   // "."
        while i < len { out.appendByte(UInt8(from: digits(i) + 48)); i = i + 1 }
    };
    out
}

/// Scientific rendering of a shortest digit sequence: "d0.d1…e±exp", or "d0e±exp"
/// when there is a single digit. e.g. "1e20", "1.234568e8".
func floatShortestSciString(sr: FloatSig, upper: Bool) -> String {
    let digits = sr.digits;
    var out = String();
    out.appendByte(UInt8(from: digits(0) + 48));
    if digits.count > 1 {
        out.appendByte(UInt8(from: 46));   // "."
        var j: Int64 = 1;
        while j < digits.count { out.appendByte(UInt8(from: digits(j) + 48)); j = j + 1 }
    };
    appendExponent(out, sr.decExp, upper)
}

/// Scientific string of `m × 2^e` (> 0) with `precision` fractional mantissa
/// digits, e.g. "9.223372e18", "4.940656e-324". `upper` selects 'E'.
func floatSciString(m: UInt64, e: Int64, precision: Int64, upper: Bool) -> String {
    let sr = floatSigDigits(m, e, precision + 1);
    let digits = sr.digits;
    var out = String();
    out.appendByte(UInt8(from: digits(0) + 48));
    if precision > 0 {
        out.appendByte(UInt8(from: 46));   // '.'
        var j: Int64 = 1;
        while j <= precision {
            let d = if j < digits.count { digits(j) } else { 0 };
            out.appendByte(UInt8(from: d + 48));
            j = j + 1
        }
    };
    appendExponent(out, sr.decExp, upper)
}
