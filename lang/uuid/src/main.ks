// UUID generation and parsing.

module uuid

import std.numeric.(UInt32, UInt64, Int64, RandomNumberGenerator)
import std.text.(String, StringBuilder, Char, CharsView, Formattable, FormatOptions)
import std.core.(Equatable, Hashable, Hasher, Matchable, Bool)
import uuid.secure_random.(SecureRandom)

/// A universally unique identifier (RFC 9562).
///
/// Stored as two 64-bit integers representing the upper and lower halves
/// of the 128-bit value. Supports generation (v4 random), formatting to
/// the canonical `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` form, and parsing
/// from that form.
///
/// # Examples
///
/// ```
/// let id = UUID.v4();
/// let s = id.formatted();
/// ```
public struct UUID: Equatable, Hashable, Formattable, Matchable {
    var high: UInt64
    var low: UInt64

    /// Creates a UUID from its raw 128-bit halves.
    public init(high high: UInt64, low low: UInt64) {
        self.high = high;
        self.low = low;
    }

    /// The nil UUID (all zeros).
    public static var nil: UUID { UUID(high: 0, low: 0) }

    /// Returns true if this is the nil UUID.
    public var isNil: Bool { self.high == 0 and self.low == 0 }

    // ========================================================================
    // GENERATION
    // ========================================================================

    /// Generates a random UUID (version 4, variant 1) using the OS
    /// cryptographic random source.
    public static func v4() -> UUID {
        var rng = SecureRandom();
        UUID.v4(using: rng)
    }

    /// Generates a random UUID (version 4, variant 1) using the given
    /// random number generator.
    public static func v4[R](mutating using rng: R) -> UUID where R: RandomNumberGenerator {
        var high = rng.nextUInt64();
        var low = rng.nextUInt64();

        high = (high & 0xFFFFFFFFFFFF0FFF) | 0x4000;  // version 4
        low = (low & 0x3FFFFFFFFFFFFFFF) | 0x8000000000000000;  // variant 1

        UUID(high: high, low: low)
    }

    // ========================================================================
    // FORMATTING
    // ========================================================================

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        // 8-4-4-4-12 hex groups separated by dashes
        let a = self.high >> 32;
        let b = (self.high >> 16) & 0xFFFF;
        let c = self.high & 0xFFFF;
        let d = self.low >> 48;
        let e = self.low & 0xFFFFFFFFFFFF;
        writer.append("\(a:08x)-\(b:04x)-\(c:04x)-\(d:04x)-\(e:012x)")
    }

    public func isEqual(to other: UUID) -> Bool {
        self.high == other.high and self.low == other.low
    }

    public func matches(other: UUID) -> Bool {
        self.isEqual(to: other)
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.high.hash(into: hasher);
        self.low.hash(into: hasher)
    }

    // ========================================================================
    // PARSING
    // ========================================================================

    /// Parses a UUID from `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
    /// Returns null if the format is invalid.
    public init(from string: String)? {
        let chars = string.chars;
        if chars.count != 36 { return null; }

        // Validate dash positions
        if chars(8) != '-' { return null; }
        if chars(13) != '-' { return null; }
        if chars(18) != '-' { return null; }
        if chars(23) != '-' { return null; }

        // Parse hex groups: 8-4-4 into high, 4-12 into low
        guard let .Some(g1) = parseHexRun(chars, from: 0, to: 8) else { return null; }
        guard let .Some(g2) = parseHexRun(chars, from: 9, to: 13) else { return null; }
        guard let .Some(g3) = parseHexRun(chars, from: 14, to: 18) else { return null; }
        guard let .Some(g4) = parseHexRun(chars, from: 19, to: 23) else { return null; }
        guard let .Some(g5) = parseHexRun(chars, from: 24, to: 36) else { return null; }

        self.high = (g1 << 32) | (g2 << 16) | g3;
        self.low = (g4 << 48) | g5;
    }
}

// Hex digit value lookup: index by (charValue - '0'). Covers '0'..'f' (55 entries).
// Values 0-15 are valid nibbles; 255 marks an invalid entry.
let HEX_TABLE: [UInt32] = [
    0,  1,  2,  3,  4,  5,  6,  7,  8,  9,        // '0'-'9'
    255, 255, 255, 255, 255, 255, 255,              // ':'-'@'
    10, 11, 12, 13, 14, 15,                         // 'A'-'F'
    255, 255, 255, 255, 255, 255, 255, 255,         // 'G'-'N'
    255, 255, 255, 255, 255, 255, 255, 255,         // 'O'-'V'
    255, 255, 255, 255, 255, 255, 255, 255, 255,    // 'W'-'_'
    255,                                             // '`'
    10, 11, 12, 13, 14, 15                          // 'a'-'f'
]

// Parses a run of hex chars from a CharsView into a UInt64.
func parseHexRun(chars: CharsView, from start: Int64, to end: Int64) -> UInt64? {
    var result: UInt64 = 0;
    var i = start;
    while i < end {
        guard let .Some(nibble) = hexDigitValue(chars(i)) else { return .None; }
        result = (result << 4) | UInt64(from: nibble);
        i = i + 1;
    }
    .Some(result)
}

func hexDigitValue(ch: Char) -> UInt32? {
    let v = ch.value();
    if v < 0x30 or v > 0x66 { return .None; }
    let nibble = HEX_TABLE(Int64(from: v - 0x30));
    if nibble == 255 { return .None; }
    .Some(nibble)
}
