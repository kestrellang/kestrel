#!/usr/bin/env python3
"""
Generates Unicode lookup tables for Kestrel from Unicode Character Database files.

Generates:
- case_tables.ks - uppercase/lowercase/titlecase mappings
- case_folding.ks - case folding for case-insensitive comparison
- grapheme_tables.ks - grapheme cluster break properties (UAX #29)

Usage:
    python generate_tables.py

Requires data files from fetch_unicode_data.py.
"""

import os
from pathlib import Path
from dataclasses import dataclass
from typing import Optional

UNICODE_VERSION = "15.1.0"
BLOCK_SIZE = 256  # Two-stage lookup: codepoint >> 8 -> block, then block[codepoint & 0xFF]

# ============================================================================
# DATA STRUCTURES
# ============================================================================

@dataclass
class CaseMapping:
    """Case mapping for a single code point."""
    upper: Optional[int] = None  # Simple uppercase mapping (single char)
    lower: Optional[int] = None  # Simple lowercase mapping (single char)
    title: Optional[int] = None  # Simple titlecase mapping (single char)


@dataclass
class SpecialCase:
    """Special case mapping (1-to-many)."""
    codepoint: int
    upper: list[int]  # Can be multiple code points
    lower: list[int]
    title: list[int]


# ============================================================================
# PARSING
# ============================================================================

def parse_unicode_data(path: Path) -> dict[int, CaseMapping]:
    """Parse UnicodeData.txt for simple case mappings."""
    mappings: dict[int, CaseMapping] = {}

    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue

            fields = line.split(";")
            if len(fields) < 14:
                continue

            codepoint = int(fields[0], 16)
            upper_str = fields[12].strip()
            lower_str = fields[13].strip()

            # Titlecase is field 14 if present, otherwise same as uppercase
            title_str = fields[14].strip() if len(fields) > 14 else upper_str

            mapping = CaseMapping()
            if upper_str:
                mapping.upper = int(upper_str, 16)
            if lower_str:
                mapping.lower = int(lower_str, 16)
            if title_str:
                mapping.title = int(title_str, 16)

            if mapping.upper or mapping.lower or mapping.title:
                mappings[codepoint] = mapping

    return mappings


def parse_special_casing(path: Path) -> list[SpecialCase]:
    """Parse SpecialCasing.txt for 1-to-many case mappings."""
    cases: list[SpecialCase] = []

    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            # Remove comments
            if "#" in line:
                line = line[:line.index("#")]
            line = line.strip()
            if not line:
                continue

            fields = [f.strip() for f in line.split(";")]
            if len(fields) < 4:
                continue

            # Skip conditional mappings (have conditions in field 4)
            if len(fields) > 4 and fields[4]:
                continue

            codepoint = int(fields[0], 16)
            lower = [int(x, 16) for x in fields[1].split()] if fields[1] else [codepoint]
            title = [int(x, 16) for x in fields[2].split()] if fields[2] else [codepoint]
            upper = [int(x, 16) for x in fields[3].split()] if fields[3] else [codepoint]

            # Only include if it's actually a 1-to-many mapping
            if len(upper) > 1 or len(lower) > 1 or len(title) > 1:
                cases.append(SpecialCase(codepoint, upper, lower, title))

    return cases


def parse_case_folding(path: Path) -> dict[int, list[int]]:
    """Parse CaseFolding.txt for case folding mappings."""
    folding: dict[int, list[int]] = {}

    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            # Remove comments
            if "#" in line:
                line = line[:line.index("#")]
            line = line.strip()
            if not line:
                continue

            fields = [f.strip() for f in line.split(";")]
            if len(fields) < 3:
                continue

            codepoint = int(fields[0], 16)
            status = fields[1]

            # C = common, F = full (includes multi-char)
            # S = simple (single char only) - skip as C covers it
            # T = Turkic special case - skip
            if status not in ("C", "F"):
                continue

            fold = [int(x, 16) for x in fields[2].split()]
            folding[codepoint] = fold

    return folding


def parse_grapheme_break_property(path: Path) -> dict[int, str]:
    """Parse GraphemeBreakProperty.txt for grapheme cluster boundaries."""
    properties: dict[int, str] = {}

    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            # Remove comments
            if "#" in line:
                line = line[:line.index("#")]
            line = line.strip()
            if not line:
                continue

            fields = [f.strip() for f in line.split(";")]
            if len(fields) < 2:
                continue

            # Parse code point or range
            cp_range = fields[0].strip()
            prop = fields[1].strip()

            if ".." in cp_range:
                start, end = cp_range.split("..")
                start_cp = int(start, 16)
                end_cp = int(end, 16)
                for cp in range(start_cp, end_cp + 1):
                    properties[cp] = prop
            else:
                properties[int(cp_range, 16)] = prop

    return properties


# ============================================================================
# TWO-STAGE TABLE GENERATION
# ============================================================================

def build_two_stage_table(
    data: dict[int, int],
    default: int = 0,
    max_codepoint: int = 0x10FFFF
) -> tuple[list[int], list[list[int]]]:
    """
    Build a two-stage lookup table.

    Returns (stage1, stage2) where:
    - stage1[codepoint >> 8] = block index
    - stage2[block_index][codepoint & 0xFF] = value
    """
    num_blocks = (max_codepoint + BLOCK_SIZE) // BLOCK_SIZE

    # Build all blocks
    blocks: list[list[int]] = []
    for block_num in range(num_blocks):
        block: list[int] = []
        for offset in range(BLOCK_SIZE):
            cp = block_num * BLOCK_SIZE + offset
            block.append(data.get(cp, default))
        blocks.append(block)

    # Deduplicate blocks
    unique_blocks: list[list[int]] = []
    block_to_index: dict[tuple, int] = {}
    stage1: list[int] = []

    for block in blocks:
        block_tuple = tuple(block)
        if block_tuple not in block_to_index:
            block_to_index[block_tuple] = len(unique_blocks)
            unique_blocks.append(block)
        stage1.append(block_to_index[block_tuple])

    return stage1, unique_blocks


# ============================================================================
# CODE GENERATION
# ============================================================================

def generate_case_tables(
    mappings: dict[int, CaseMapping],
    special_cases: list[SpecialCase],
    output_path: Path
) -> None:
    """Generate case_tables.ks with case mapping tables and functions."""

    # Build delta tables (value - codepoint for simple mappings)
    upper_deltas: dict[int, int] = {}
    lower_deltas: dict[int, int] = {}
    title_deltas: dict[int, int] = {}

    for cp, mapping in mappings.items():
        if mapping.upper is not None and mapping.upper != cp:
            upper_deltas[cp] = mapping.upper - cp
        if mapping.lower is not None and mapping.lower != cp:
            lower_deltas[cp] = mapping.lower - cp
        if mapping.title is not None and mapping.title != cp:
            title_deltas[cp] = mapping.title - cp

    # Build two-stage tables
    upper_stage1, upper_stage2 = build_two_stage_table(upper_deltas)
    lower_stage1, lower_stage2 = build_two_stage_table(lower_deltas)
    title_stage1, title_stage2 = build_two_stage_table(title_deltas)

    # Generate Kestrel code
    lines = [
        "// Unicode case mapping tables",
        f"// Generated from Unicode {UNICODE_VERSION} - DO NOT EDIT",
        "// Run generate_tables.py to regenerate",
        "",
        "module std.text.unicode",
        "",
        "import std.text.(Char, String)",
        "import std.num.(Int64, Int32, UInt32)",
        "import std.core.(Bool)",
        "",
        f'/// The Unicode version these tables were generated from.',
        f'public let unicodeVersion: String = "{UNICODE_VERSION}"',
        "",
    ]

    # Generate uppercase tables
    lines.extend(_generate_table_arrays("UPPER", upper_stage1, upper_stage2))
    lines.append("")

    # Generate lowercase tables
    lines.extend(_generate_table_arrays("LOWER", lower_stage1, lower_stage2))
    lines.append("")

    # Generate titlecase tables
    lines.extend(_generate_table_arrays("TITLE", title_stage1, title_stage2))
    lines.append("")

    # Generate special cases array
    lines.extend(_generate_special_cases(special_cases))
    lines.append("")

    # Generate lookup functions
    lines.extend(_generate_case_functions())

    output_path.write_text("\n".join(lines))
    print(f"  Generated {output_path} ({len(lines)} lines)")


def _generate_table_arrays(prefix: str, stage1: list[int], stage2: list[list[int]]) -> list[str]:
    """Generate Kestrel array literals for two-stage table."""
    lines = []

    # Stage 1 (block indices)
    lines.append(f"/// {prefix} case mapping stage 1 table (block indices)")
    lines.append(f"let {prefix}_STAGE1: [Int32] = [")
    for i in range(0, len(stage1), 16):
        chunk = stage1[i:i+16]
        lines.append("    " + ", ".join(str(x) for x in chunk) + ",")
    lines.append("]")
    lines.append("")

    # Stage 2 (blocks with deltas)
    lines.append(f"/// {prefix} case mapping stage 2 table (delta values)")
    lines.append(f"let {prefix}_STAGE2: [Int32] = [")
    for block_idx, block in enumerate(stage2):
        lines.append(f"    // Block {block_idx}")
        for i in range(0, len(block), 16):
            chunk = block[i:i+16]
            lines.append("    " + ", ".join(str(x) for x in chunk) + ",")
    lines.append("]")

    return lines


def _generate_special_cases(special_cases: list[SpecialCase]) -> list[str]:
    """Generate special case mappings array."""
    lines = []

    # Filter to only uppercase expansions for now (most common need: ß -> SS)
    upper_expansions = [(sc.codepoint, sc.upper) for sc in special_cases if len(sc.upper) > 1]
    lower_expansions = [(sc.codepoint, sc.lower) for sc in special_cases if len(sc.lower) > 1]
    title_expansions = [(sc.codepoint, sc.title) for sc in special_cases if len(sc.title) > 1]

    # TODO: Enable when tuple array types are supported
    # Special case mappings (1-to-many) are disabled due to tuple array type issues
    lines.append("// Special case mappings (1-to-many) - disabled until tuple arrays are supported")
    lines.append("")

    return lines


def _generate_case_functions() -> list[str]:
    """Generate case conversion lookup functions."""
    lines = []

    lines.append("// ============================================================================")
    lines.append("// CASE CONVERSION FUNCTIONS")
    lines.append("// ============================================================================")
    lines.append("")

    # toUppercase
    lines.append("/// Returns the uppercase version of a character.")
    lines.append("/// For characters with multi-char expansions, returns the first char.")
    lines.append("/// Use hasUppercaseExpansion() and uppercaseExpansion() for full support.")
    lines.append("public func toUppercase(c: Char) -> Char {")
    lines.append("    let cp = c.value();")
    lines.append("    // ASCII fast path")
    lines.append("    if cp >= 97 and cp <= 122 {")
    lines.append("        return Char(cp - 32)")
    lines.append("    }")
    lines.append("    if cp > 0x10FFFF { return c }")
    lines.append("    let blockIdx = UPPER_STAGE1(unchecked: Int64(from: cp.shiftRight(by: 8)));")
    lines.append("    let delta = UPPER_STAGE2(unchecked: Int64(from: blockIdx) * 256 + Int64(from: cp.bitwiseAnd(UInt32(intLiteral: 0xFF))));")
    lines.append("    if delta == 0 { return c }")
    lines.append("    Char(UInt32(from: Int64(from: cp).add(Int64(from: delta))))")
    lines.append("}")
    lines.append("")

    # toLowercase
    lines.append("/// Returns the lowercase version of a character.")
    lines.append("public func toLowercase(c: Char) -> Char {")
    lines.append("    let cp = c.value();")
    lines.append("    // ASCII fast path")
    lines.append("    if cp >= 65 and cp <= 90 {")
    lines.append("        return Char(cp + 32)")
    lines.append("    }")
    lines.append("    if cp > 0x10FFFF { return c }")
    lines.append("    let blockIdx = LOWER_STAGE1(unchecked: Int64(from: cp.shiftRight(by: 8)));")
    lines.append("    let delta = LOWER_STAGE2(unchecked: Int64(from: blockIdx) * 256 + Int64(from: cp.bitwiseAnd(UInt32(intLiteral: 0xFF))));")
    lines.append("    if delta == 0 { return c }")
    lines.append("    Char(UInt32(from: Int64(from: cp).add(Int64(from: delta))))")
    lines.append("}")
    lines.append("")

    # toTitlecase
    lines.append("/// Returns the titlecase version of a character.")
    lines.append("public func toTitlecase(c: Char) -> Char {")
    lines.append("    let cp = c.value();")
    lines.append("    // ASCII fast path (same as uppercase)")
    lines.append("    if cp >= 97 and cp <= 122 {")
    lines.append("        return Char(cp - 32)")
    lines.append("    }")
    lines.append("    if cp > 0x10FFFF { return c }")
    lines.append("    let blockIdx = TITLE_STAGE1(unchecked: Int64(from: cp.shiftRight(by: 8)));")
    lines.append("    let delta = TITLE_STAGE2(unchecked: Int64(from: blockIdx) * 256 + Int64(from: cp.bitwiseAnd(UInt32(intLiteral: 0xFF))));")
    lines.append("    if delta == 0 { return c }")
    lines.append("    Char(UInt32(from: Int64(from: cp).add(Int64(from: delta))))")
    lines.append("}")
    lines.append("")

    # hasUppercaseExpansion
    lines.append("/// Returns true if the character has a multi-character uppercase expansion.")
    lines.append("public func hasUppercaseExpansion(c: Char) -> Bool {")
    lines.append("    let cp = c.value();")
    lines.append("    var i: Int64 = 0;")
    lines.append("    while i < UPPER_EXPANSIONS_COUNT {")
    lines.append("        let entry = UPPER_EXPANSIONS(unchecked: i);")
    lines.append("        if UInt32(from: entry.0) == cp { return true }")
    lines.append("        i = i + 1")
    lines.append("    }")
    lines.append("    false")
    lines.append("}")
    lines.append("")

    # uppercaseExpansion
    lines.append("/// Returns the multi-character uppercase expansion for a character.")
    lines.append("/// Returns empty string if no expansion exists.")
    lines.append("public func uppercaseExpansion(c: Char) -> String {")
    lines.append("    let cp = c.value();")
    lines.append("    var i: Int64 = 0;")
    lines.append("    while i < UPPER_EXPANSIONS_COUNT {")
    lines.append("        let entry = UPPER_EXPANSIONS(unchecked: i);")
    lines.append("        if UInt32(from: entry.0) == cp {")
    lines.append("            var result = String();")
    lines.append("            result.appendChar(Char(UInt32(from: entry.2)));")
    lines.append("            if entry.1 >= 2 { result.appendChar(Char(UInt32(from: entry.3))) }")
    lines.append("            if entry.1 >= 3 { result.appendChar(Char(UInt32(from: entry.4))) }")
    lines.append("            return result")
    lines.append("        }")
    lines.append("        i = i + 1")
    lines.append("    }")
    lines.append('    ""')
    lines.append("}")
    lines.append("")

    # Similar functions for lowercase and titlecase
    lines.append("/// Returns true if the character has a multi-character lowercase expansion.")
    lines.append("public func hasLowercaseExpansion(c: Char) -> Bool {")
    lines.append("    let cp = c.value();")
    lines.append("    var i: Int64 = 0;")
    lines.append("    while i < LOWER_EXPANSIONS_COUNT {")
    lines.append("        let entry = LOWER_EXPANSIONS(unchecked: i);")
    lines.append("        if UInt32(from: entry.0) == cp { return true }")
    lines.append("        i = i + 1")
    lines.append("    }")
    lines.append("    false")
    lines.append("}")
    lines.append("")

    lines.append("/// Returns the multi-character lowercase expansion for a character.")
    lines.append("public func lowercaseExpansion(c: Char) -> String {")
    lines.append("    let cp = c.value();")
    lines.append("    var i: Int64 = 0;")
    lines.append("    while i < LOWER_EXPANSIONS_COUNT {")
    lines.append("        let entry = LOWER_EXPANSIONS(unchecked: i);")
    lines.append("        if UInt32(from: entry.0) == cp {")
    lines.append("            var result = String();")
    lines.append("            result.appendChar(Char(UInt32(from: entry.2)));")
    lines.append("            if entry.1 >= 2 { result.appendChar(Char(UInt32(from: entry.3))) }")
    lines.append("            if entry.1 >= 3 { result.appendChar(Char(UInt32(from: entry.4))) }")
    lines.append("            return result")
    lines.append("        }")
    lines.append("        i = i + 1")
    lines.append("    }")
    lines.append('    ""')
    lines.append("}")
    lines.append("")

    lines.append("/// Returns true if the character has a multi-character titlecase expansion.")
    lines.append("public func hasTitlecaseExpansion(c: Char) -> Bool {")
    lines.append("    let cp = c.value();")
    lines.append("    var i: Int64 = 0;")
    lines.append("    while i < TITLE_EXPANSIONS_COUNT {")
    lines.append("        let entry = TITLE_EXPANSIONS(unchecked: i);")
    lines.append("        if UInt32(from: entry.0) == cp { return true }")
    lines.append("        i = i + 1")
    lines.append("    }")
    lines.append("    false")
    lines.append("}")
    lines.append("")

    lines.append("/// Returns the multi-character titlecase expansion for a character.")
    lines.append("public func titlecaseExpansion(c: Char) -> String {")
    lines.append("    let cp = c.value();")
    lines.append("    var i: Int64 = 0;")
    lines.append("    while i < TITLE_EXPANSIONS_COUNT {")
    lines.append("        let entry = TITLE_EXPANSIONS(unchecked: i);")
    lines.append("        if UInt32(from: entry.0) == cp {")
    lines.append("            var result = String();")
    lines.append("            result.appendChar(Char(UInt32(from: entry.2)));")
    lines.append("            if entry.1 >= 2 { result.appendChar(Char(UInt32(from: entry.3))) }")
    lines.append("            if entry.1 >= 3 { result.appendChar(Char(UInt32(from: entry.4))) }")
    lines.append("            return result")
    lines.append("        }")
    lines.append("        i = i + 1")
    lines.append("    }")
    lines.append('    ""')
    lines.append("}")

    return lines


def generate_case_folding(
    folding: dict[int, list[int]],
    output_path: Path
) -> None:
    """Generate case_folding.ks with case folding tables and functions."""

    # Separate simple (single char) and full (multi-char) foldings
    simple_folding: dict[int, int] = {}
    full_folding: list[tuple[int, list[int]]] = []

    for cp, fold in folding.items():
        if len(fold) == 1:
            if fold[0] != cp:
                simple_folding[cp] = fold[0] - cp  # Store as delta
        else:
            full_folding.append((cp, fold))

    # Build two-stage table for simple folding
    stage1, stage2 = build_two_stage_table(simple_folding)

    lines = [
        "// Unicode case folding tables",
        f"// Generated from Unicode {UNICODE_VERSION} - DO NOT EDIT",
        "// Run generate_tables.py to regenerate",
        "",
        "module std.text.unicode",
        "",
        "import std.text.(Char, String)",
        "import std.num.(Int64, Int32, UInt32)",
        "import std.core.(Bool)",
        "",
    ]

    # Generate simple folding tables
    lines.extend(_generate_table_arrays("FOLD", stage1, stage2))
    lines.append("")

    # Generate full folding array
    lines.append(f"/// Full case folding expansions: (codepoint, length, char1, char2, char3)")
    lines.append(f"let FOLD_EXPANSIONS_COUNT: Int64 = {len(full_folding)};")
    lines.append(f"let FOLD_EXPANSIONS: [(Int32, Int32, Int32, Int32, Int32)] = [")
    for cp, fold in full_folding:
        f = fold + [0] * (3 - len(fold))
        lines.append(f"    ({cp}, {len(fold)}, {f[0]}, {f[1]}, {f[2]}),  // U+{cp:04X}")
    lines.append("]")
    lines.append("")

    # Generate lookup functions
    lines.append("// ============================================================================")
    lines.append("// CASE FOLDING FUNCTIONS")
    lines.append("// ============================================================================")
    lines.append("")

    lines.append("/// Returns the case-folded version of a character (for case-insensitive comparison).")
    lines.append("/// For characters with multi-char folding, returns the first char.")
    lines.append("public func caseFold(c: Char) -> Char {")
    lines.append("    let cp = c.value();")
    lines.append("    // ASCII fast path")
    lines.append("    if cp >= 65 and cp <= 90 {")
    lines.append("        return Char(cp + 32)")
    lines.append("    }")
    lines.append("    if cp > 0x10FFFF { return c }")
    lines.append("    let blockIdx = FOLD_STAGE1(unchecked: Int64(from: cp.shiftRight(by: 8)));")
    lines.append("    let delta = FOLD_STAGE2(unchecked: Int64(from: blockIdx) * 256 + Int64(from: cp.bitwiseAnd(UInt32(intLiteral: 0xFF))));")
    lines.append("    if delta == 0 { return c }")
    lines.append("    Char(UInt32(from: Int64(from: cp).add(Int64(from: delta))))")
    lines.append("}")
    lines.append("")

    lines.append("/// Returns true if the character has a multi-character case fold.")
    lines.append("public func hasCaseFoldExpansion(c: Char) -> Bool {")
    lines.append("    let cp = c.value();")
    lines.append("    var i: Int64 = 0;")
    lines.append("    while i < FOLD_EXPANSIONS_COUNT {")
    lines.append("        let entry = FOLD_EXPANSIONS(unchecked: i);")
    lines.append("        if UInt32(from: entry.0) == cp { return true }")
    lines.append("        i = i + 1")
    lines.append("    }")
    lines.append("    false")
    lines.append("}")
    lines.append("")

    lines.append("/// Returns the multi-character case fold for a character.")
    lines.append("public func caseFoldExpansion(c: Char) -> String {")
    lines.append("    let cp = c.value();")
    lines.append("    var i: Int64 = 0;")
    lines.append("    while i < FOLD_EXPANSIONS_COUNT {")
    lines.append("        let entry = FOLD_EXPANSIONS(unchecked: i);")
    lines.append("        if UInt32(from: entry.0) == cp {")
    lines.append("            var result = String();")
    lines.append("            result.appendChar(Char(UInt32(from: entry.2)));")
    lines.append("            if entry.1 >= 2 { result.appendChar(Char(UInt32(from: entry.3))) }")
    lines.append("            if entry.1 >= 3 { result.appendChar(Char(UInt32(from: entry.4))) }")
    lines.append("            return result")
    lines.append("        }")
    lines.append("        i = i + 1")
    lines.append("    }")
    lines.append('    ""')
    lines.append("}")

    output_path.write_text("\n".join(lines))
    print(f"  Generated {output_path} ({len(lines)} lines)")


def generate_grapheme_tables(
    properties: dict[int, str],
    output_path: Path
) -> None:
    """Generate grapheme_tables.ks with grapheme break property tables."""

    # Map property names to integers
    PROPERTY_MAP = {
        "Other": 0,
        "CR": 1,
        "LF": 2,
        "Control": 3,
        "Extend": 4,
        "ZWJ": 5,
        "Regional_Indicator": 6,
        "Prepend": 7,
        "SpacingMark": 8,
        "L": 9,
        "V": 10,
        "T": 11,
        "LV": 12,
        "LVT": 13,
    }

    # Convert properties to integers
    prop_ints: dict[int, int] = {}
    for cp, prop in properties.items():
        prop_ints[cp] = PROPERTY_MAP.get(prop, 0)

    # Build two-stage table
    stage1, stage2 = build_two_stage_table(prop_ints)

    lines = [
        "// Unicode grapheme break property tables",
        f"// Generated from Unicode {UNICODE_VERSION} - DO NOT EDIT",
        "// Run generate_tables.py to regenerate",
        "",
        "module std.text.unicode",
        "",
        "import std.text.(Char)",
        "import std.num.(Int64, Int32, UInt32)",
        "import std.core.(Bool, Equatable, Matchable)",
        "",
        "// ============================================================================",
        "// GRAPHEME BREAK PROPERTY ENUM",
        "// ============================================================================",
        "",
        "/// Grapheme cluster break property (UAX #29).",
        "public enum GraphemeBreakProperty: Equatable, Matchable {",
        "    case Other",
        "    case CR",
        "    case LF",
        "    case Control",
        "    case Extend",
        "    case ZWJ",
        "    case RegionalIndicator",
        "    case Prepend",
        "    case SpacingMark",
        "    case L    // Hangul leading jamo",
        "    case V    // Hangul vowel jamo",
        "    case T    // Hangul trailing jamo",
        "    case LV   // Hangul LV syllable",
        "    case LVT  // Hangul LVT syllable",
        "",
        "    public func isEqual(to other: GraphemeBreakProperty) -> Bool {",
        "        self.ordinal() == other.ordinal()",
        "    }",
        "",
        "    public func matches(other: GraphemeBreakProperty) -> Bool {",
        "        self.ordinal() == other.ordinal()",
        "    }",
        "",
        "    func ordinal() -> Int32 {",
        "        match self {",
        "            .Other => 0,",
        "            .CR => 1,",
        "            .LF => 2,",
        "            .Control => 3,",
        "            .Extend => 4,",
        "            .ZWJ => 5,",
        "            .RegionalIndicator => 6,",
        "            .Prepend => 7,",
        "            .SpacingMark => 8,",
        "            .L => 9,",
        "            .V => 10,",
        "            .T => 11,",
        "            .LV => 12,",
        "            .LVT => 13",
        "        }",
        "    }",
        "}",
        "",
    ]

    # Generate tables
    lines.extend(_generate_table_arrays("GBP", stage1, stage2))
    lines.append("")

    # Generate lookup function
    lines.append("// ============================================================================")
    lines.append("// GRAPHEME BREAK PROPERTY LOOKUP")
    lines.append("// ============================================================================")
    lines.append("")

    lines.append("/// Returns the grapheme break property for a character.")
    lines.append("public func graphemeBreakProperty(c: Char) -> GraphemeBreakProperty {")
    lines.append("    let cp = c.value();")
    lines.append("    if cp > 0x10FFFF { return .Other }")
    lines.append("    let blockIdx = GBP_STAGE1(unchecked: Int64(from: cp.shiftRight(by: 8)));")
    lines.append("    let prop = GBP_STAGE2(unchecked: Int64(from: blockIdx) * 256 + Int64(from: cp.bitwiseAnd(UInt32(intLiteral: 0xFF))));")
    lines.append("    match prop {")
    lines.append("        0 => .Other,")
    lines.append("        1 => .CR,")
    lines.append("        2 => .LF,")
    lines.append("        3 => .Control,")
    lines.append("        4 => .Extend,")
    lines.append("        5 => .ZWJ,")
    lines.append("        6 => .RegionalIndicator,")
    lines.append("        7 => .Prepend,")
    lines.append("        8 => .SpacingMark,")
    lines.append("        9 => .L,")
    lines.append("        10 => .V,")
    lines.append("        11 => .T,")
    lines.append("        12 => .LV,")
    lines.append("        13 => .LVT,")
    lines.append("        _ => .Other")
    lines.append("    }")
    lines.append("}")
    lines.append("")

    # Generate break rule function (UAX #29 state machine)
    # Use full type names for comparisons (Kestrel requires this)
    GBP = "GraphemeBreakProperty"
    lines.append("/// Returns true if there should be a grapheme cluster break between")
    lines.append("/// two characters with the given properties.")
    lines.append("/// Implements UAX #29 grapheme cluster boundary rules.")
    lines.append("public func shouldBreakBetween(")
    lines.append("    prev: GraphemeBreakProperty,")
    lines.append("    curr: GraphemeBreakProperty,")
    lines.append("    prevPrevWasRI: Bool,")
    lines.append("    prevWasZWJ: Bool")
    lines.append(") -> Bool {")
    lines.append("    // GB1, GB2: Break at start/end of text (handled externally)")
    lines.append("")
    lines.append("    // GB3: Do not break between CR and LF")
    lines.append(f"    if prev == {GBP}.CR and curr == {GBP}.LF {{ return false }}")
    lines.append("")
    lines.append("    // GB4: Break after Control, CR, LF")
    lines.append(f"    if prev == {GBP}.Control or prev == {GBP}.CR or prev == {GBP}.LF {{ return true }}")
    lines.append("")
    lines.append("    // GB5: Break before Control, CR, LF")
    lines.append(f"    if curr == {GBP}.Control or curr == {GBP}.CR or curr == {GBP}.LF {{ return true }}")
    lines.append("")
    lines.append("    // GB6: Do not break Hangul syllable sequences (L + L/V/LV/LVT)")
    lines.append(f"    if prev == {GBP}.L and (curr == {GBP}.L or curr == {GBP}.V or curr == {GBP}.LV or curr == {GBP}.LVT) {{")
    lines.append("        return false")
    lines.append("    }")
    lines.append("")
    lines.append("    // GB7: Do not break Hangul syllable sequences (LV/V + V/T)")
    lines.append(f"    if (prev == {GBP}.LV or prev == {GBP}.V) and (curr == {GBP}.V or curr == {GBP}.T) {{")
    lines.append("        return false")
    lines.append("    }")
    lines.append("")
    lines.append("    // GB8: Do not break Hangul syllable sequences (LVT/T + T)")
    lines.append(f"    if (prev == {GBP}.LVT or prev == {GBP}.T) and curr == {GBP}.T {{")
    lines.append("        return false")
    lines.append("    }")
    lines.append("")
    lines.append("    // GB9: Do not break before Extend or ZWJ")
    lines.append(f"    if curr == {GBP}.Extend or curr == {GBP}.ZWJ {{ return false }}")
    lines.append("")
    lines.append("    // GB9a: Do not break before SpacingMark")
    lines.append(f"    if curr == {GBP}.SpacingMark {{ return false }}")
    lines.append("")
    lines.append("    // GB9b: Do not break after Prepend")
    lines.append(f"    if prev == {GBP}.Prepend {{ return false }}")
    lines.append("")
    lines.append("    // GB11: Do not break within emoji ZWJ sequences")
    lines.append("    // (Extended_Pictographic Extend* ZWJ) x Extended_Pictographic")
    lines.append("    // Simplified: ZWJ followed by anything doesn't break")
    lines.append("    if prevWasZWJ { return false }")
    lines.append("")
    lines.append("    // GB12, GB13: Do not break within emoji flag sequences")
    lines.append("    // (Regional_Indicator Regional_Indicator) forms a flag")
    lines.append(f"    if prev == {GBP}.RegionalIndicator and curr == {GBP}.RegionalIndicator {{")
    lines.append("        // Only pair up: break if we already have a pair")
    lines.append("        return prevPrevWasRI")
    lines.append("    }")
    lines.append("")
    lines.append("    // GB999: Otherwise, break everywhere")
    lines.append("    true")
    lines.append("}")

    output_path.write_text("\n".join(lines))
    print(f"  Generated {output_path} ({len(lines)} lines)")


# ============================================================================
# MAIN
# ============================================================================

def main():
    script_dir = Path(__file__).parent
    data_dir = script_dir / "data"

    # Check data files exist
    required_files = ["UnicodeData.txt", "SpecialCasing.txt", "CaseFolding.txt", "GraphemeBreakProperty.txt"]
    for name in required_files:
        if not (data_dir / name).exists():
            print(f"Error: {name} not found in {data_dir}")
            print("Run fetch_unicode_data.py first.")
            return 1

    print(f"Generating Unicode {UNICODE_VERSION} tables...")
    print()

    # Parse data files
    print("Parsing UnicodeData.txt...")
    case_mappings = parse_unicode_data(data_dir / "UnicodeData.txt")
    print(f"  Found {len(case_mappings)} case mappings")

    print("Parsing SpecialCasing.txt...")
    special_cases = parse_special_casing(data_dir / "SpecialCasing.txt")
    print(f"  Found {len(special_cases)} special cases")

    print("Parsing CaseFolding.txt...")
    case_folding = parse_case_folding(data_dir / "CaseFolding.txt")
    print(f"  Found {len(case_folding)} case foldings")

    print("Parsing GraphemeBreakProperty.txt...")
    grapheme_props = parse_grapheme_break_property(data_dir / "GraphemeBreakProperty.txt")
    print(f"  Found {len(grapheme_props)} grapheme properties")

    print()
    print("Generating Kestrel source files...")

    # Generate output files
    generate_case_tables(case_mappings, special_cases, script_dir / "case_tables.ks")
    generate_case_folding(case_folding, script_dir / "case_folding.ks")
    generate_grapheme_tables(grapheme_props, script_dir / "grapheme_tables.ks")

    print()
    print("Done!")
    return 0


if __name__ == "__main__":
    exit(main())
