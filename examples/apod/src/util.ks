// Utilities: minimal URL encoding and date validation.

module apod.util

/// Percent-encodes a string for use in a URL query.
///
/// Operates byte-level so non-ASCII UTF-8 round-trips correctly via
/// `%XX` escapes for each byte.
public func urlEncode(s: String) -> String {
    var out = String();
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(i);
        if (b >= 65 and b <= 90) or (b >= 97 and b <= 122) or (b >= 48 and b <= 57) or b == 45 or b == 95 or b == 46 {
            out.appendByte(b)
        } else {
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

/// Returns true if `s` looks like an ISO date "YYYY-MM-DD". The NASA endpoint
/// rejects anything else, so we filter at the edge to keep the request URL clean.
public func isIsoDate(s: String) -> Bool {
    if s.byteCount != 10 { return false };
    var i: Int64 = 0;
    while i < 10 {
        let c = s.chars(i);
        if i == 4 or i == 7 {
            if c != '-' { return false }
        } else {
            if not c.isAsciiDigit { return false }
        };
        i = i + 1
    }
    true
}
