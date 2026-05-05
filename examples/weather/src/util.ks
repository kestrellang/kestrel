// Utilities: URL encoding/decoding, form parsing, number formatting

module weather.util

// ============================================================================
// URL ENCODING
// ============================================================================

public func urlEncode(s: String) -> String {
    var out = String();
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(unchecked: i);
        if b == 32 {
            out.appendByte(43)  // space -> +
        } else if (b >= 65 and b <= 90) or (b >= 97 and b <= 122) or (b >= 48 and b <= 57) or b == 45 or b == 95 or b == 46 {
            out.appendByte(b)
        } else {
            // percent-encode
            out.appendByte(37); // %
            let hi = Int64(from: b) / 16;
            let lo = Int64(from: b) % 16;
            if hi < 10 { out.appendByte(UInt8(from: 48 + hi)) } else { out.appendByte(UInt8(from: 55 + hi)) };
            if lo < 10 { out.appendByte(UInt8(from: 48 + lo)) } else { out.appendByte(UInt8(from: 55 + lo)) }
        };
        i = i + 1
    }
    out
}

public func urlDecode(s: String) -> String {
    var out = String();
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(unchecked: i);
        if b == 43 {
            out.appendByte(32)  // + -> space
        } else if b == 37 and i + 2 < len {
            let hi = hexVal(s.bytes(unchecked: i + 1));
            let lo = hexVal(s.bytes(unchecked: i + 2));
            out.appendByte(hi * 16 + lo);
            i = i + 2
        } else {
            out.appendByte(b)
        };
        i = i + 1
    }
    out
}

public func hexVal(b: UInt8) -> UInt8 {
    if b >= 48 and b <= 57 { return b - 48 }
    if b >= 65 and b <= 70 { return b - 55 }
    if b >= 97 and b <= 102 { return b - 87 }
    0
}

// ============================================================================
// FORM PARSING
// ============================================================================

public func parseFormValue(body: String, key: String) -> String {
    // Parse "city=Berlin&other=val" style form data
    let keyEq = key + "=";
    let keyLen = keyEq.byteCount;
    let bodyLen = body.byteCount;

    // Find key= in the body
    var pos: Int64 = 0;
    while pos <= bodyLen - keyLen {
        var matched = true;
        var ki: Int64 = 0;
        while ki < keyLen {
            if body.bytes(unchecked: pos + ki) != keyEq.bytes(unchecked: ki) {
                matched = false;
                break
            };
            ki = ki + 1
        }
        if matched and (pos == 0 or body.bytes(unchecked: pos - 1) == 38) {
            // Found it — extract value until & or end
            let valStart = pos + keyLen;
            var valEnd = valStart;
            while valEnd < bodyLen and body.bytes(unchecked: valEnd) != 38 {
                valEnd = valEnd + 1
            }
            return urlDecode(body.asSlice().subslice(from: valStart, to: valEnd).toOwned())
        };
        pos = pos + 1
    }
    ""
}

// ============================================================================
// NUMBER FORMATTING
// ============================================================================

public func formatTemp(t: Float64) -> String {
    // Format to 1 decimal place using integer math
    let negative = t < 0.0;
    let absVal = if negative { 0.0 - t } else { t };
    let scaled = absVal * 10.0 + 0.5; // round
    let total = match scaled.toInt64() {
        .Some(n) => n,
        .None => 0
    };
    let whole = total / 10;
    let frac = total % 10;
    var s = String();
    if negative and total > 0 {
        s.append("-")
    };
    s.append(whole.format());
    s.append(".");
    s.append(frac.format());
    s
}

public func formatInt(n: Int64) -> String {
    n.format()
}

public func formatTempWhole(t: Float64) -> String {
    let negative = t < 0.0;
    let absVal = if negative { 0.0 - t } else { t };
    let rounded = absVal + 0.5;
    let whole = match rounded.toInt64() {
        .Some(n) => n,
        .None => 0
    };
    var s = String();
    if negative and whole > 0 {
        s.append("-")
    };
    s.append(whole.format());
    s
}
