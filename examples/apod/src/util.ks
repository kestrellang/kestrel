// Utilities: date validation.

module apod.util

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
