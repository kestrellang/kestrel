// Utilities: comma-separated guess lists, parsing.

module wordle.util

public func splitGuesses(s: String) -> Array[String] {
    var out = Array[String]();
    if s.byteCount == 0 { return out };
    var start: Int64 = 0;
    var i: Int64 = 0;
    while i < s.byteCount {
        if s.bytes(unchecked: i) == 44 {
            // ',' separator
            if i > start {
                out.append(s.asSlice().subslice(from: start, to: i).toOwned())
            };
            start = i + 1
        };
        i = i + 1
    }
    if i > start {
        out.append(s.asSlice().subslice(from: start, to: i).toOwned())
    };
    out
}

public func joinGuesses(guesses: Array[String]) -> String {
    var out = String();
    var i: Int64 = 0;
    while i < guesses.count {
        if i > 0 { out.append(",") };
        out.append(guesses(unchecked: i));
        i = i + 1
    }
    out
}

/// Parse a non-negative seed, falling back to 1 on bad input.
public func parseSeed(s: String) -> Int64 {
    match Int64(parsing: s) {
        .Some(n) => if n > 0 { n } else { 1 },
        .None => 1
    }
}

/// Pseudo-random seed from the address of a fresh allocation. We don't
/// need cryptographic randomness — just something that varies per run.
public func nextSeed(prev: Int64) -> Int64 {
    let x = prev * 1103515245 + 12345;
    let masked = x % 2147483647;
    if masked < 0 { 0 - masked } else { masked }
}

/// Validate and normalize a guess: must be 5 ASCII letters; returns
/// uppercase form, or empty string on rejection.
public func normalizeGuess(raw: String) -> String {
    if raw.byteCount != 5 { return "" };
    var i: Int64 = 0;
    while i < 5 {
        let b = raw.bytes(unchecked: i);
        let isLower = b >= 97 and b <= 122;
        let isUpper = b >= 65 and b <= 90;
        if not (isLower or isUpper) { return "" };
        i = i + 1
    }
    raw.uppercasedAscii()
}
