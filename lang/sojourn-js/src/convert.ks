module js

// Conversions between JS strings (UTF-16 code units), Kestrel strings
// and numbers, and the number formatting and parsing rules of
// Number::toString and StringToNumber.

/// A JS string's contents.
type Units = Array[UInt16];

/// UTF-16 code units of a Kestrel string.
func unitsOf(s: String) -> Units {
    var out: Units = [];
    for c in s.chars {
        let v = Int64(from: c.value());
        if v >= 0x10000 {
            let x = v - 0x10000;
            out.append(UInt16(truncating: 0xD800 + (x >> 10)));
            out.append(UInt16(truncating: 0xDC00 + (x & 0x3FF)));
        } else {
            out.append(UInt16(truncating: v));
        }
    }
    out
}

/// A Kestrel string from code units. Lone surrogates stay, WTF-8 encoded
/// (see `appendCodePoint`).
func stringOf(units: Units) -> String {
    var out = "";
    var i: Int64 = 0;
    while i < units.count {
        let u = Int64(from: units(i));
        if u >= 0xD800 and u <= 0xDBFF and i + 1 < units.count {
            let lo = Int64(from: units(i + 1));
            if lo >= 0xDC00 and lo <= 0xDFFF {
                appendCodePoint(out, 0x10000 + ((u - 0xD800) << 10) + (lo - 0xDC00));
                i = i + 2;
                continue;
            }
        }
        appendCodePoint(out, u);
        i = i + 1;
    }
    out
}

func unitsEqual(a: Units, b: Units) -> Bool {
    if a.count != b.count { return false; }
    for i in 0..<a.count {
        if a(i) != b(i) { return false; }
    }
    true
}

/// Compares code unit by code unit, as `<` does on strings.
func unitsLess(a: Units, b: Units) -> Bool {
    let n = if a.count < b.count { a.count } else { b.count };
    for i in 0..<n {
        if a(i) != b(i) { return a(i) < b(i); }
    }
    a.count < b.count
}

func unitsSlice(u: Units, from: Int64, to: Int64) -> Units {
    var out: Units = [];
    var i = from;
    while i < to {
        out.append(u(i));
        i = i + 1;
    }
    out
}

/// The index of `needle` in `hay` at or after `from`, or -1.
func unitsIndexOf(hay: Units, needle: Units, from: Int64) -> Int64 {
    var i = if from < 0 { 0 } else { from };
    while i + needle.count <= hay.count {
        var ok = true;
        for j in 0..<needle.count {
            if hay(i + j) != needle(j) {
                ok = false;
                break;
            }
        }
        if ok { return i; }
        i = i + 1;
    }
    return -1;
}

func isJSWhitespace(u: UInt16) -> Bool {
    let c = Int64(from: u);
    c == 9 or c == 10 or c == 11 or c == 12 or c == 13 or c == 32 or c == 0xA0 or c == 0x1680
        or (c >= 0x2000 and c <= 0x200A) or c == 0x2028 or c == 0x2029 or c == 0x202F or c == 0x205F
        or c == 0x3000 or c == 0xFEFF
}

func unitsTrim(u: Units, start: Bool, end: Bool) -> Units {
    var a: Int64 = 0;
    var b = u.count;
    if start {
        while a < b and isJSWhitespace(u(a)) { a = a + 1; }
    }
    if end {
        while b > a and isJSWhitespace(u(b - 1)) { b = b - 1; }
    }
    unitsSlice(u, a, b)
}

// ----------------------------------------------------------------------
// Numbers
// ----------------------------------------------------------------------

/// Number::toString(10): the shortest digits that round-trip, laid out
/// the way JS does (`1e+21`, `0.000001`, `1e-7`).
func numberToString(n: Float64) -> String {
    if n.isNaN { return "NaN"; }
    if n == 0.0 { return "0"; }
    if n.isInfinite { return if n > 0.0 { "Infinity" } else { "-Infinity" }; }
    if n < 0.0 { return "-\(numberToString(0.0 - n))"; }
    let (d, point) = shortestDigits(n);
    formatDigits(d, d.bytes.count, point)
}

/// The shortest round-trip digits of a positive finite number, without
/// leading or trailing zeros, and how many come before the point: `n`
/// is 0.d × 10^point.
func shortestDigits(n: Float64) -> (String, Int64) {
    // Kestrel prints the shortest round-trip digits, as `123.45` or `1.2e-7`.
    let text = "\(n)";
    var mantissa = "";
    var exponent: Int64 = 0;
    var inExponent = false;
    var expText = "";
    for c in text.chars {
        if c == 'e' or c == 'E' {
            inExponent = true;
            continue;
        }
        if inExponent { expText.append(char: c); } else { mantissa.append(char: c); }
    }
    if inExponent { exponent = parseSmallInt(expText); }
    // Digits without the point, and where the point goes.
    var digits = "";
    var point: Int64 = -1;
    var count: Int64 = 0;
    for c in mantissa.chars {
        if c == '.' {
            point = count;
            continue;
        }
        digits.append(char: c);
        count = count + 1;
    }
    if point < 0 { point = count; }
    point = point + exponent;
    // Drop leading and trailing zeros.
    var ds: [Char] = [];
    for c in digits.chars { ds.append(c); }
    var lead: Int64 = 0;
    while lead < ds.count - 1 and ds(lead) == '0' {
        lead = lead + 1;
        point = point - 1;
    }
    var end = ds.count;
    while end > lead + 1 and ds(end - 1) == '0' { end = end - 1; }
    var d = "";
    for i in lead..<end { d.append(char: ds(i)); }
    (d, point)
}

/// Lays out digits `d` (`k` of them) with the decimal point after `n` of
/// them, per Number::toString.
func formatDigits(d: String, k: Int64, n: Int64) -> String {
    if k <= n and n <= 21 { return "\(d)\("0".repeated(n - k))"; }
    var chars: [Char] = [];
    for c in d.chars { chars.append(c); }
    if 0 < n and n <= 21 {
        var out = "";
        for i in 0..<n { out.append(char: chars(i)); }
        out.append(char: '.');
        for i in n..<k { out.append(char: chars(i)); }
        return out;
    }
    if -6 < n and n <= 0 { return "0.\("0".repeated(0 - n))\(d)"; }
    let e = n - 1;
    let sign = if e >= 0 { "+" } else { "-" };
    let mag = if e >= 0 { e } else { 0 - e };
    if k == 1 { return "\(d)e\(sign)\(mag)"; }
    var out = "";
    out.append(char: chars(0));
    out.append(char: '.');
    for i in 1..<k { out.append(char: chars(i)); }
    "\(out)e\(sign)\(mag)"
}

func parseSmallInt(s: String) -> Int64 {
    var n: Int64 = 0;
    var negative = false;
    for c in s.chars {
        if c == '-' { negative = true; continue; }
        if c == '+' { continue; }
        n = n * 10 + Int64(from: c.value()) - 48;
    }
    if negative { 0 - n } else { n }
}

/// Number::toString(radix) for radix other than 10.
func numberToRadixString(n: Float64, radix: Int64) -> String {
    if n.isNaN { return "NaN"; }
    if n.isInfinite { return if n > 0.0 { "Infinity" } else { "-Infinity" }; }
    if n == 0.0 { return "0"; }
    let negative = n < 0.0;
    var x = if negative { 0.0 - n } else { n };
    var intPart = x.floor();
    var frac = x - intPart;
    let digits = "0123456789abcdefghijklmnopqrstuvwxyz";
    var dchars: [Char] = [];
    for c in digits.chars { dchars.append(c); }
    let r = Float64(from: radix);
    var intDigits: [Char] = [];
    if intPart == 0.0 { intDigits.append('0'); }
    while intPart >= 1.0 {
        let q = (intPart / r).floor();
        let digit = (intPart - q * r).toInt64() ?? 0;
        intDigits.append(dchars(digit));
        intPart = q;
    }
    var out = if negative { "-" } else { "" };
    var i = intDigits.count - 1;
    while i >= 0 {
        out.append(char: intDigits(i));
        i = i - 1;
    }
    if frac > 0.0 {
        out.append(char: '.');
        var count = 0;
        while frac > 0.0 and count < 52 {
            frac = frac * r;
            let digit = frac.floor();
            out.append(char: dchars(digit.toInt64() ?? 0));
            frac = frac - digit;
            count = count + 1;
        }
    }
    let _ = x;
    out
}

/// StringToNumber: whitespace-trimmed decimal, hex, octal or binary
/// literal text, or NaN.
func stringToNumber(u: Units) -> Float64 {
    let t = unitsTrim(u, true, true);
    if t.count == 0 { return 0.0; }
    let s = stringOf(t);
    if s == "Infinity" or s == "+Infinity" { return Float64.infinity; }
    if s == "-Infinity" { return 0.0 - Float64.infinity; }
    var chars: [Char] = [];
    for c in s.chars { chars.append(c); }
    if chars.count > 2 and chars(0) == '0' {
        let p = chars(1);
        let radix: Int64 = if p == 'x' or p == 'X' { 16 } else if p == 'o' or p == 'O' { 8 } else if p == 'b' or p == 'B' { 2 } else { 0 };
        if radix > 0 {
            var value = 0.0;
            for i in 2..<chars.count {
                let d = digitValue(chars(i));
                if d < 0 or d >= radix { return Float64.nan; }
                value = value * Float64(from: radix) + Float64(from: d);
            }
            return value;
        }
    }
    if not isDecimalLiteral(chars) { return Float64.nan; }
    Float64(parsing: s) ?? Float64.nan
}

/// The value of a digit in bases up to 36, or -1.
func digitValue(c: Char) -> Int64 {
    let v = Int64(from: c.value());
    if v >= 48 and v <= 57 { return v - 48; }
    if v >= 97 and v <= 122 { return v - 97 + 10; }
    if v >= 65 and v <= 90 { return v - 65 + 10; }
    return -1;
}

/// `[+-] (digits [. digits?] | . digits) [(e|E) [+-] digits]`
func isDecimalLiteral(c: Array[Char]) -> Bool {
    var i: Int64 = 0;
    if i < c.count and (c(i) == '+' or c(i) == '-') { i = i + 1; }
    var digits = 0;
    while i < c.count and isDigitChar(c(i)) { i = i + 1; digits = digits + 1; }
    if i < c.count and c(i) == '.' {
        i = i + 1;
        while i < c.count and isDigitChar(c(i)) { i = i + 1; digits = digits + 1; }
    }
    if digits == 0 { return false; }
    if i < c.count and (c(i) == 'e' or c(i) == 'E') {
        i = i + 1;
        if i < c.count and (c(i) == '+' or c(i) == '-') { i = i + 1; }
        var expDigits = 0;
        while i < c.count and isDigitChar(c(i)) { i = i + 1; expDigits = expDigits + 1; }
        if expDigits == 0 { return false; }
    }
    i == c.count
}

func isDigitChar(c: Char) -> Bool {
    let v = c.value();
    v >= 48 and v <= 57
}

/// ToInt32's wrapping: the integer modulo 2^32, as a signed value.
func toInt32(n: Float64) -> Int64 {
    if n.isNaN or n.isInfinite { return 0; }
    let t = n.trunc();
    let two32 = 4294967296.0;
    var m = t - (t / two32).floor() * two32;
    if m >= 2147483648.0 { m = m - two32; }
    m.toInt64() ?? 0
}

func toUint32(n: Float64) -> Int64 {
    let i = toInt32(n);
    if i < 0 { i + 4294967296 } else { i }
}

/// The canonical array index a string names, or -1: "0", "1", … up to
/// 2^32 - 2, with no leading zeros.
func arrayIndexOf(s: String) -> Int64 {
    var n: Int64 = 0;
    var count: Int64 = 0;
    var first = '0';
    for c in s.chars {
        if not isDigitChar(c) { return -1; }
        if count == 0 { first = c; }
        n = n * 10 + Int64(from: c.value()) - 48;
        count = count + 1;
        if count > 10 { return -1; }
    }
    if count == 0 { return -1; }
    if count > 1 and first == '0' { return -1; }
    if n > 4294967294 { return -1; }
    n
}
