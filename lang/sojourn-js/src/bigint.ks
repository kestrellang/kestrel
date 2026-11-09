module js

// Arbitrary-precision integers for BigInt: a sign and a magnitude of
// 32-bit limbs, least significant first, with no leading zero limbs.
// Zero has no limbs and is never negative. Everything here is pure; the
// VM glue (heap encoding, conversions, built-ins) is natives_bigint.ks.

struct BigNum {
    var neg: Bool
    var mag: [UInt32]
}

func bnZero() -> BigNum { BigNum(neg: false, mag: []) }

func bnIsZero(a: BigNum) -> Bool { a.mag.isEmpty }

/// Drops leading zero limbs, and the sign of zero.
func bnMake(neg: Bool, mag: Array[UInt32]) -> BigNum {
    var m = mag;
    while m.count > 0 and m(m.count - 1) == 0 { let _ = m.pop(); }
    BigNum(neg: neg and m.count > 0, mag: m)
}

func bnFromU64(u: UInt64, neg: Bool) -> BigNum {
    bnMake(neg, [UInt32(truncating: u), UInt32(truncating: u >> 32)])
}

func bnFromInt(n: Int64) -> BigNum {
    if n >= 0 { return bnFromU64(UInt64(bitPattern: n), false); }
    // Two's complement negation, which also covers Int64's minimum.
    let u = (UInt64(bitPattern: n) ^ 0xFFFFFFFFFFFFFFFF) + 1;
    bnFromU64(u, true)
}

func bnNeg(a: BigNum) -> BigNum { bnMake(not a.neg, a.mag) }

func bnAbs(a: BigNum) -> BigNum { BigNum(neg: false, mag: a.mag) }

// ----------------------------------------------------------------------
// Magnitudes
// ----------------------------------------------------------------------

func magCmp(a: Array[UInt32], b: Array[UInt32]) -> Int64 {
    if a.count != b.count { return if a.count < b.count { -1 } else { 1 }; }
    var i = a.count - 1;
    while i >= 0 {
        if a(i) != b(i) { return if a(i) < b(i) { -1 } else { 1 }; }
        i = i - 1;
    }
    0
}

func limb(a: Array[UInt32], i: Int64) -> UInt64 {
    if i < a.count { UInt64(from: a(i)) } else { 0 }
}

func magAdd(a: Array[UInt32], b: Array[UInt32]) -> Array[UInt32] {
    var out: [UInt32] = [];
    var carry: UInt64 = 0;
    let n = if a.count > b.count { a.count } else { b.count };
    for i in 0..<n {
        let s = limb(a, i) + limb(b, i) + carry;
        out.append(UInt32(truncating: s));
        carry = s >> 32;
    }
    if carry > 0 { out.append(UInt32(truncating: carry)); }
    out
}

/// a - b, for a >= b.
func magSub(a: Array[UInt32], b: Array[UInt32]) -> Array[UInt32] {
    var out: [UInt32] = [];
    var borrow: Int64 = 0;
    for i in 0..<a.count {
        var d = Int64(from: a(i)) - Int64(bitPattern: limb(b, i)) - borrow;
        if d < 0 {
            d = d + 4294967296;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.append(UInt32(truncating: UInt64(bitPattern: d)));
    }
    while out.count > 0 and out(out.count - 1) == 0 { let _ = out.pop(); }
    out
}

func magMul(a: Array[UInt32], b: Array[UInt32]) -> Array[UInt32] {
    if a.isEmpty or b.isEmpty { return []; }
    var out: [UInt32] = [];
    for _ in 0..<(a.count + b.count) { out.append(0); }
    for i in 0..<a.count {
        var carry: UInt64 = 0;
        let x = UInt64(from: a(i));
        for j in 0..<b.count {
            let t = UInt64(from: out(i + j)) + x * UInt64(from: b(j)) + carry;
            out(i + j) = UInt32(truncating: t);
            carry = t >> 32;
        }
        out(i + b.count) = UInt32(truncating: carry);
    }
    while out.count > 0 and out(out.count - 1) == 0 { let _ = out.pop(); }
    out
}

/// a * m + add, for small m and add.
func magMulAdd(a: Array[UInt32], m: UInt64, add: UInt64) -> Array[UInt32] {
    var out: [UInt32] = [];
    var carry = add;
    for x in a {
        let t = UInt64(from: x) * m + carry;
        out.append(UInt32(truncating: t));
        carry = t >> 32;
    }
    while carry > 0 {
        out.append(UInt32(truncating: carry));
        carry = carry >> 32;
    }
    out
}

/// a / d and a % d, for a one-limb d.
func magDivSmall(a: Array[UInt32], d: UInt64) -> (Array[UInt32], UInt64) {
    var q: [UInt32] = [];
    for _ in 0..<a.count { q.append(0); }
    var rem: UInt64 = 0;
    var i = a.count - 1;
    while i >= 0 {
        let cur = (rem << 32) | UInt64(from: a(i));
        q(i) = UInt32(truncating: cur / d);
        rem = cur % d;
        i = i - 1;
    }
    while q.count > 0 and q(q.count - 1) == 0 { let _ = q.pop(); }
    (q, rem)
}

func clz32(x: UInt32) -> Int64 {
    if x == 0 { return 32; }
    var n: Int64 = 0;
    var v = UInt64(from: x);
    while v & 0x80000000 == 0 {
        v = v << 1;
        n = n + 1;
    }
    n
}

func magBitLength(a: Array[UInt32]) -> Int64 {
    if a.isEmpty { return 0; }
    (a.count - 1) * 32 + (32 - clz32(a(a.count - 1)))
}

func magShl(a: Array[UInt32], n: Int64) -> Array[UInt32] {
    if a.isEmpty { return []; }
    let limbs = n / 32;
    let bits = n % 32;
    var out: [UInt32] = [];
    for _ in 0..<limbs { out.append(0); }
    var carry: UInt64 = 0;
    for x in a {
        let t = (UInt64(from: x) << bits) | carry;
        out.append(UInt32(truncating: t));
        carry = if bits == 0 { 0 } else { UInt64(from: x) >> (32 - bits) };
    }
    if carry > 0 { out.append(UInt32(truncating: carry)); }
    out
}

func magShr(a: Array[UInt32], n: Int64) -> Array[UInt32] {
    let limbs = n / 32;
    let bits = n % 32;
    if limbs >= a.count { return []; }
    var out: [UInt32] = [];
    for i in limbs..<a.count {
        var t = UInt64(from: a(i)) >> bits;
        if bits > 0 and i + 1 < a.count { t = t | ((UInt64(from: a(i + 1)) << (32 - bits)) & 0xFFFFFFFF); }
        out.append(UInt32(truncating: t));
    }
    while out.count > 0 and out(out.count - 1) == 0 { let _ = out.pop(); }
    out
}

/// Long division (Knuth's algorithm D): quotient and remainder.
func magDivMod(a: Array[UInt32], b: Array[UInt32]) -> (Array[UInt32], Array[UInt32]) {
    if magCmp(a, b) < 0 { return ([], a); }
    if b.count == 1 {
        let (q, r) = magDivSmall(a, UInt64(from: b(0)));
        return (q, if r == 0 { [] } else { [UInt32(truncating: r)] });
    }
    // Normalize so the divisor's top limb has its high bit set.
    let s = clz32(b(b.count - 1));
    let bn = magShl(b, s);
    var an = magShl(a, s);
    while an.count <= a.count { an.append(0); }
    let n = bn.count;
    let m = an.count - n;
    var q: [UInt32] = [];
    for _ in 0..<m { q.append(0); }
    let top = UInt64(from: bn(n - 1));
    let next = UInt64(from: bn(n - 2));
    var j = m - 1;
    while j >= 0 {
        let num = (UInt64(from: an(j + n)) << 32) | UInt64(from: an(j + n - 1));
        var qhat = num / top;
        var rhat = num % top;
        while qhat >= 4294967296 or qhat * next > ((rhat << 32) | UInt64(from: an(j + n - 2))) {
            qhat = qhat - 1;
            rhat = rhat + top;
            if rhat >= 4294967296 { break; }
        }
        // an[j..j+n] -= qhat * bn.
        var borrow: Int64 = 0;
        var carry: UInt64 = 0;
        for i in 0..<n {
            let p = qhat * UInt64(from: bn(i)) + carry;
            carry = p >> 32;
            let t = Int64(from: an(i + j)) - borrow - Int64(bitPattern: p & 0xFFFFFFFF);
            an(i + j) = UInt32(truncating: UInt64(bitPattern: t));
            borrow = if t < 0 { 1 } else { 0 };
        }
        let t = Int64(from: an(j + n)) - borrow - Int64(bitPattern: carry);
        an(j + n) = UInt32(truncating: UInt64(bitPattern: t));
        if t < 0 {
            // qhat was one too big: add the divisor back.
            qhat = qhat - 1;
            var c: UInt64 = 0;
            for i in 0..<n {
                let s2 = UInt64(from: an(i + j)) + UInt64(from: bn(i)) + c;
                an(i + j) = UInt32(truncating: s2);
                c = s2 >> 32;
            }
            an(j + n) = UInt32(truncating: UInt64(from: an(j + n)) + c);
        }
        q(j) = UInt32(truncating: qhat);
        j = j - 1;
    }
    var r: [UInt32] = [];
    for i in 0..<n { r.append(an(i)); }
    while q.count > 0 and q(q.count - 1) == 0 { let _ = q.pop(); }
    (q, magShr(r, s))
}

// ----------------------------------------------------------------------
// Arithmetic
// ----------------------------------------------------------------------

func bnCmp(a: BigNum, b: BigNum) -> Int64 {
    if a.neg != b.neg { return if a.neg { -1 } else { 1 }; }
    let c = magCmp(a.mag, b.mag);
    if a.neg { 0 - c } else { c }
}

func bnEq(a: BigNum, b: BigNum) -> Bool { bnCmp(a, b) == 0 }

func bnAdd(a: BigNum, b: BigNum) -> BigNum {
    if a.neg == b.neg { return bnMake(a.neg, magAdd(a.mag, b.mag)); }
    let c = magCmp(a.mag, b.mag);
    if c == 0 { return bnZero(); }
    if c > 0 { return bnMake(a.neg, magSub(a.mag, b.mag)); }
    bnMake(b.neg, magSub(b.mag, a.mag))
}

func bnSub(a: BigNum, b: BigNum) -> BigNum { bnAdd(a, bnNeg(b)) }

func bnMul(a: BigNum, b: BigNum) -> BigNum { bnMake(a.neg != b.neg, magMul(a.mag, b.mag)) }

/// Truncating division; the divisor isn't zero.
func bnDiv(a: BigNum, b: BigNum) -> BigNum {
    let (q, _) = magDivMod(a.mag, b.mag);
    bnMake(a.neg != b.neg, q)
}

/// The remainder of truncating division: its sign follows `a`.
func bnRem(a: BigNum, b: BigNum) -> BigNum {
    let (_, r) = magDivMod(a.mag, b.mag);
    bnMake(a.neg, r)
}

/// a ** e for a non-negative exponent that fits an Int64.
func bnPow(a: BigNum, e: Int64) -> BigNum {
    var result = bnFromInt(1);
    var base = a;
    var k = e;
    while k > 0 {
        if k % 2 == 1 { result = bnMul(result, base); }
        k = k / 2;
        if k > 0 { base = bnMul(base, base); }
    }
    result
}

/// a << n (n may be negative, shifting right). Right shifts round
/// toward negative infinity.
func bnShl(a: BigNum, n: Int64) -> BigNum {
    if n >= 0 { return bnMake(a.neg, magShl(a.mag, n)); }
    let k = 0 - n;
    if not a.neg { return bnMake(false, magShr(a.mag, k)); }
    // floor(-x / 2^k) = -((x - 1) >> k) - 1
    let one: [UInt32] = [1];
    let shifted = magShr(magSub(a.mag, one), k);
    bnMake(true, magAdd(shifted, one))
}

// ----------------------------------------------------------------------
// Two's complement, for the bitwise operators and asIntN
// ----------------------------------------------------------------------

/// `a` in `n`-limb two's complement.
func bnToTwos(a: BigNum, n: Int64) -> Array[UInt32] {
    var out: [UInt32] = [];
    for i in 0..<n { out.append(if i < a.mag.count { a.mag(i) } else { 0 }); }
    if not a.neg { return out; }
    var carry: UInt64 = 1;
    for i in 0..<n {
        let t = (UInt64(from: out(i)) ^ 0xFFFFFFFF) + carry;
        out(i) = UInt32(truncating: t);
        carry = t >> 32;
    }
    out
}

/// A two's complement limb array back to a BigNum (the top bit is the sign).
func bnFromTwos(t: Array[UInt32]) -> BigNum {
    if t.isEmpty { return bnZero(); }
    if UInt64(from: t(t.count - 1)) & 0x80000000 == 0 { return bnMake(false, t); }
    var out: [UInt32] = [];
    var carry: UInt64 = 1;
    for x in t {
        let v = (UInt64(from: x) ^ 0xFFFFFFFF) + carry;
        out.append(UInt32(truncating: v));
        carry = v >> 32;
    }
    bnMake(true, out)
}

/// `&` (0), `|` (1) or `^` (2).
func bnBitwise(a: BigNum, b: BigNum, op: Int64) -> BigNum {
    let n = (if a.mag.count > b.mag.count { a.mag.count } else { b.mag.count }) + 1;
    let x = bnToTwos(a, n);
    let y = bnToTwos(b, n);
    var out: [UInt32] = [];
    for i in 0..<n {
        let p = UInt64(from: x(i));
        let q = UInt64(from: y(i));
        let r = if op == 0 { p & q } else if op == 1 { p | q } else { p ^ q };
        out.append(UInt32(truncating: r));
    }
    bnFromTwos(out)
}

/// BigInt.asUintN: `a` modulo 2^bits.
func bnAsUintN(bits: Int64, a: BigNum) -> BigNum {
    if bits == 0 { return bnZero(); }
    if not a.neg and magBitLength(a.mag) <= bits { return a; }
    let limbs = (bits + 31) / 32;
    let n = if limbs > a.mag.count + 1 { limbs } else { a.mag.count + 1 };
    var t = bnToTwos(a, n);
    while t.count > limbs { let _ = t.pop(); }
    let extra = limbs * 32 - bits;
    if extra > 0 {
        let full: UInt64 = 4294967295;
        let mask = full >> extra;
        t(limbs - 1) = UInt32(truncating: UInt64(from: t(limbs - 1)) & mask);
    }
    bnMake(false, t)
}

/// BigInt.asIntN: `a` wrapped to a signed `bits`-bit integer.
func bnAsIntN(bits: Int64, a: BigNum) -> BigNum {
    if bits == 0 { return bnZero(); }
    // Anything already in range is unchanged.
    if magBitLength(a.mag) < bits { return a; }
    let u = bnAsUintN(bits, a);
    if magBitLength(u.mag) < bits { return u; }
    bnSub(u, bnMake(false, magShl(oneMag(), bits)))
}

// ----------------------------------------------------------------------
// Numbers and strings
// ----------------------------------------------------------------------

/// An integral double, exactly.
func bnFromFloat(x: Float64) -> BigNum {
    if x == 0.0 { return bnZero(); }
    let bits = x.bitPattern;
    let neg = bits >> 63 == 1;
    let expField = Int64(bitPattern: (bits >> 52) & 0x7FF);
    var mant = bits & 0xFFFFFFFFFFFFF;
    var e = expField - 1075;
    if expField == 0 { e = 0 - 1074; } else { mant = mant | 0x10000000000000; }
    let m = bnFromU64(mant, false);
    let v = bnShl(m, e);
    if neg { bnNeg(v) } else { v }
}

/// The nearest double (ties to even).
func bnToFloat(a: BigNum) -> Float64 {
    let len = magBitLength(a.mag);
    if len == 0 { return 0.0; }
    var x = 0.0;
    if len <= 64 {
        x = u64ToFloat((limb(a.mag, 1) << 32) | limb(a.mag, 0));
    } else {
        // The top 64 bits, with any lower bit folded into the last one
        // so rounding still sees it.
        let shift = len - 64;
        let top = magShr(a.mag, shift);
        var t = (limb(top, 1) << 32) | limb(top, 0);
        let back = magShl(top, shift);
        if magCmp(back, a.mag) != 0 { t = t | 1; }
        if shift > 1023 { return if a.neg { 0.0 - Float64.infinity } else { Float64.infinity }; }
        x = u64ToFloat(t) * Float64(bitPattern: UInt64(bitPattern: shift + 1023) << 52);
    }
    if a.neg { 0.0 - x } else { x }
}

/// Compares `a` with a finite double: -1, 0 or 1.
func bnCmpFloat(a: BigNum, x: Float64) -> Int64 {
    let f = x.floor();
    let c = bnCmp(a, bnFromFloat(f));
    if c != 0 { return c; }
    if x > f { -1 } else { 0 }
}

func digitChar(d: Int64) -> Int64 { if d < 10 { 48 + d } else { 87 + d } }

/// The digits of `a` in `radix`, with a leading "-" if negative.
func bnToString(a: BigNum, radix: Int64) -> String {
    if bnIsZero(a) { return "0"; }
    // Peel off as many digits per division as fit in one limb.
    var chunk = radix;
    var per: Int64 = 1;
    while chunk * radix < 4294967296 {
        chunk = chunk * radix;
        per = per + 1;
    }
    var digits: [Int64] = [];
    var m = a.mag;
    while not m.isEmpty {
        let (q, r) = magDivSmall(m, UInt64(bitPattern: chunk));
        var rem = Int64(bitPattern: r);
        for _ in 0..<per {
            digits.append(rem % radix);
            rem = rem / radix;
        }
        m = q;
    }
    while digits.count > 1 and digits(digits.count - 1) == 0 { let _ = digits.pop(); }
    var out = if a.neg { "-" } else { "" };
    var i = digits.count - 1;
    while i >= 0 {
        appendCodePoint(out, digitChar(digits(i)));
        i = i - 1;
    }
    out
}

func bigDigitValue(c: Int64) -> Int64 {
    if c >= 48 and c <= 57 { return c - 48; }
    if c >= 97 and c <= 122 { return c - 87; }
    if c >= 65 and c <= 90 { return c - 55; }
    99
}

/// Digits in `radix` (no sign or prefix), or None if any is invalid.
func bnParseDigits(u: Array[UInt16], start: Int64, end: Int64, radix: Int64) -> Optional[Array[UInt32]] {
    if start >= end { return .None; }
    var m: [UInt32] = [];
    for i in start..<end {
        let d = bigDigitValue(Int64(from: u(i)));
        if d >= radix { return .None; }
        m = magMulAdd(m, UInt64(bitPattern: radix), UInt64(bitPattern: d));
    }
    while m.count > 0 and m(m.count - 1) == 0 { let _ = m.pop(); }
    .Some(m)
}

func oneMag() -> Array[UInt32] { [1] }

/// A UInt64 as the nearest double: both halves are exact, so the one
/// addition rounds once.
func u64ToFloat(u: UInt64) -> Float64 {
    let hi = Float64(from: Int64(bitPattern: u >> 32)) * 4294967296.0;
    hi + Float64(from: Int64(bitPattern: u & 0xFFFFFFFF))
}
