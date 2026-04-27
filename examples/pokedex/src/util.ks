// URL encoding/decoding + small string helpers

module pokedex.util

public func urlEncode(s: String) -> String {
    var out = String();
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(unchecked: i);
        if b == 32 {
            out.appendByte(43)
        } else if (b >= 65 and b <= 90) or (b >= 97 and b <= 122) or (b >= 48 and b <= 57) or b == 45 or b == 95 or b == 46 {
            out.appendByte(b)
        } else {
            out.appendByte(37);
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
            out.appendByte(32)
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

func hexVal(b: UInt8) -> UInt8 {
    if b >= 48 and b <= 57 { return b - 48 }
    if b >= 65 and b <= 70 { return b - 55 }
    if b >= 97 and b <= 102 { return b - 87 }
    0
}

// ASCII-only lowercase byte
public func toLowerByte(b: UInt8) -> UInt8 {
    if b >= 65 and b <= 90 { return b + 32 };
    b
}

public func toLower(s: String) -> String {
    var out = String();
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        out.appendByte(toLowerByte(s.bytes(unchecked: i)));
        i = i + 1
    }
    out
}

// Case-insensitive substring match. Needle should already be lowercase.
public func containsLower(haystack: String, needleLower: String) -> Bool {
    let hLen = haystack.byteCount;
    let nLen = needleLower.byteCount;
    if nLen == 0 { return true };
    if nLen > hLen { return false };
    var i: Int64 = 0;
    while i <= hLen - nLen {
        var matched = true;
        var j: Int64 = 0;
        while j < nLen {
            let hb = toLowerByte(haystack.bytes(unchecked: i + j));
            if hb != needleLower.bytes(unchecked: j) {
                matched = false;
                break
            };
            j = j + 1
        }
        if matched { return true };
        i = i + 1
    }
    false
}

// Capitalize first letter (ASCII), used to display "bulbasaur" -> "Bulbasaur"
public func capitalize(s: String) -> String {
    let len = s.byteCount;
    if len == 0 { return s.clone() };
    var out = String();
    let first = s.bytes(unchecked: 0);
    if first >= 97 and first <= 122 {
        out.appendByte(first - 32)
    } else {
        out.appendByte(first)
    };
    var i: Int64 = 1;
    while i < len {
        out.appendByte(s.bytes(unchecked: i));
        i = i + 1
    }
    out
}

// Pad an integer with leading zeros to width N (e.g., padId(7, 3) = "007")
public func padId(n: Int64, width: Int64) -> String {
    let raw = n.format();
    if raw.byteCount >= width { return raw };
    var out = String();
    var pad = width - raw.byteCount;
    while pad > 0 {
        out.appendByte(48); // '0'
        pad = pad - 1
    }
    out.append(raw);
    out
}

// Convert decimeters (PokeAPI height unit) to a "X.X m" string
public func formatMeters(decimeters: Int64) -> String {
    let whole = decimeters / 10;
    let frac = decimeters - whole * 10;
    var s = String();
    s.append(whole.format());
    s.append(".");
    s.append(frac.format());
    s.append(" m");
    s
}

// Convert hectograms (PokeAPI weight unit) to a "X.X kg" string
public func formatKilos(hectograms: Int64) -> String {
    let whole = hectograms / 10;
    let frac = hectograms - whole * 10;
    var s = String();
    s.append(whole.format());
    s.append(".");
    s.append(frac.format());
    s.append(" kg");
    s
}
