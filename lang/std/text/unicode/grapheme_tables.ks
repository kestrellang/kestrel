// Unicode grapheme break property tables
// Uses @fileconstant to load binary data from data/*.bin files
// Run generate_data.py to regenerate the binary files if needed

module std.text.unicode

import std.text.(Char)
import std.num.(Int64, Int32, UInt32)
import std.core.(Bool, Equatable, Matchable)
import std.memory.(LiteralSlice)

// ============================================================================
// GRAPHEME BREAK PROPERTY ENUM
// ============================================================================

/// Grapheme cluster break property (UAX #29).
public enum GraphemeBreakProperty: Equatable, Matchable {
    case Other
    case CR
    case LF
    case Control
    case Extend
    case ZWJ
    case RegionalIndicator
    case Prepend
    case SpacingMark
    case L    // Hangul leading jamo
    case V    // Hangul vowel jamo
    case T    // Hangul trailing jamo
    case LV   // Hangul LV syllable
    case LVT  // Hangul LVT syllable

    public func equals(other: GraphemeBreakProperty) -> Bool {
        self.ordinal() == other.ordinal()
    }

    public func matches(other: GraphemeBreakProperty) -> Bool {
        self.ordinal() == other.ordinal()
    }

    func ordinal() -> Int32 {
        match self {
            .Other => 0,
            .CR => 1,
            .LF => 2,
            .Control => 3,
            .Extend => 4,
            .ZWJ => 5,
            .RegionalIndicator => 6,
            .Prepend => 7,
            .SpacingMark => 8,
            .L => 9,
            .V => 10,
            .T => 11,
            .LV => 12,
            .LVT => 13
        }
    }
}

// ============================================================================
// TWO-STAGE LOOKUP TABLES (loaded from binary files)
// ============================================================================

/// GBP case mapping stage 1 table (block indices)
@fileconstant("data/gbp_stage1.bin")
let GBP_STAGE1: LiteralSlice[Int32]

/// GBP case mapping stage 2 table (property values)
@fileconstant("data/gbp_stage2.bin")
let GBP_STAGE2: LiteralSlice[Int32]

// ============================================================================
// GRAPHEME BREAK PROPERTY LOOKUP
// ============================================================================

/// Returns the grapheme break property for a character.
public func graphemeBreakProperty(c: Char) -> GraphemeBreakProperty {
    let cp = c.value();
    if cp > 0x10FFFF { return .Other }
    let blockIdx = GBP_STAGE1(unchecked: Int64(from: cp.shiftRight(by: 8)));
    let prop = GBP_STAGE2(unchecked: Int64(from: blockIdx) * 256 + Int64(from: cp.bitwiseAnd(UInt32(intLiteral: 0xFF))));
    match prop {
        0 => .Other,
        1 => .CR,
        2 => .LF,
        3 => .Control,
        4 => .Extend,
        5 => .ZWJ,
        6 => .RegionalIndicator,
        7 => .Prepend,
        8 => .SpacingMark,
        9 => .L,
        10 => .V,
        11 => .T,
        12 => .LV,
        13 => .LVT,
        _ => .Other
    }
}

/// Returns true if there should be a grapheme cluster break between
/// two characters with the given properties.
/// Implements UAX #29 grapheme cluster boundary rules.
public func shouldBreakBetween(
    prev: GraphemeBreakProperty,
    curr: GraphemeBreakProperty,
    prevPrevWasRI: Bool,
    prevWasZWJ: Bool
) -> Bool {
    // GB1, GB2: Break at start/end of text (handled externally)

    // GB3: Do not break between CR and LF
    if prev == GraphemeBreakProperty.CR and curr == GraphemeBreakProperty.LF { return false }

    // GB4: Break after Control, CR, LF
    if prev == GraphemeBreakProperty.Control or prev == GraphemeBreakProperty.CR or prev == GraphemeBreakProperty.LF { return true }

    // GB5: Break before Control, CR, LF
    if curr == GraphemeBreakProperty.Control or curr == GraphemeBreakProperty.CR or curr == GraphemeBreakProperty.LF { return true }

    // GB6: Do not break Hangul syllable sequences (L + L/V/LV/LVT)
    if prev == GraphemeBreakProperty.L and (curr == GraphemeBreakProperty.L or curr == GraphemeBreakProperty.V or curr == GraphemeBreakProperty.LV or curr == GraphemeBreakProperty.LVT) {
        return false
    }

    // GB7: Do not break Hangul syllable sequences (LV/V + V/T)
    if (prev == GraphemeBreakProperty.LV or prev == GraphemeBreakProperty.V) and (curr == GraphemeBreakProperty.V or curr == GraphemeBreakProperty.T) {
        return false
    }

    // GB8: Do not break Hangul syllable sequences (LVT/T + T)
    if (prev == GraphemeBreakProperty.LVT or prev == GraphemeBreakProperty.T) and curr == GraphemeBreakProperty.T {
        return false
    }

    // GB9: Do not break before Extend or ZWJ
    if curr == GraphemeBreakProperty.Extend or curr == GraphemeBreakProperty.ZWJ { return false }

    // GB9a: Do not break before SpacingMark
    if curr == GraphemeBreakProperty.SpacingMark { return false }

    // GB9b: Do not break after Prepend
    if prev == GraphemeBreakProperty.Prepend { return false }

    // GB11: Do not break within emoji ZWJ sequences
    // (Extended_Pictographic Extend* ZWJ) x Extended_Pictographic
    // Simplified: ZWJ followed by anything doesn't break
    if prevWasZWJ { return false }

    // GB12, GB13: Do not break within emoji flag sequences
    // (Regional_Indicator Regional_Indicator) forms a flag
    if prev == GraphemeBreakProperty.RegionalIndicator and curr == GraphemeBreakProperty.RegionalIndicator {
        // Only pair up: break if we already have a pair
        return prevPrevWasRI
    }

    // GB999: Otherwise, break everywhere
    true
}
