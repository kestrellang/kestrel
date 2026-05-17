// String helpers for the Pokédex UI

module pokedex.util

// Case-insensitive substring match. Needle should already be lowercase.
public func containsLower(haystack: String, needleLower: String) -> Bool {
    haystack.lowercasedAscii().contains(needleLower)
}

// Capitalize first letter (ASCII), used to display "bulbasaur" -> "Bulbasaur"
public func capitalize(s: String) -> String {
    let len = s.byteCount;
    if len == 0 { return s.clone() };
    let first = s.bytes(unchecked: 0);
    // If first byte is lowercase ASCII a-z, uppercase it
    if first >= 97 and first <= 122 {
        var out = String();
        out.appendChar(Char(UInt32(from: first - 32)).unwrap());
        out.append(s.asSlice().subslice(from: 1, to: len).toOwned());
        return out
    };
    s.clone()
}

// Pad an integer with leading zeros to width 3 (e.g., padId(7) = "007")
public func padId(n: Int64) -> String {
    "\(n:03)"
}

// Convert decimeters (PokeAPI height unit) to a "X.X m" string
public func formatMeters(decimeters: Int64) -> String {
    let whole = decimeters / 10;
    let frac = decimeters - whole * 10;
    "\(whole).\(frac) m"
}

// Convert hectograms (PokeAPI weight unit) to a "X.X kg" string
public func formatKilos(hectograms: Int64) -> String {
    let whole = hectograms / 10;
    let frac = hectograms - whole * 10;
    "\(whole).\(frac) kg"
}
