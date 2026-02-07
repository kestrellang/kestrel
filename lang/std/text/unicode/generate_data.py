#!/usr/bin/env python3
"""
Generates binary Unicode lookup data files for Kestrel.

Generates binary .bin files in data/ directory that are loaded at compile time
via @fileconstant attribute. The .ks source files are now static and don't need
to be regenerated.

Usage:
    python generate_data.py

Requires raw Unicode data files from fetch_unicode_data.py.
"""

import os
import struct
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
    upper: Optional[int] = None
    lower: Optional[int] = None
    title: Optional[int] = None


# ============================================================================
# PARSING (unchanged from generate_tables.py)
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


def parse_case_folding(path: Path) -> dict[int, list[int]]:
    """Parse CaseFolding.txt for case folding mappings."""
    folding: dict[int, list[int]] = {}

    with open(path, "r", encoding="utf-8") as f:
        for line in f:
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
            if "#" in line:
                line = line[:line.index("#")]
            line = line.strip()
            if not line:
                continue

            fields = [f.strip() for f in line.split(";")]
            if len(fields) < 2:
                continue

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
) -> tuple[list[int], list[int]]:
    """
    Build a two-stage lookup table.

    Returns (stage1, stage2_flat) where:
    - stage1[codepoint >> 8] = block index
    - stage2_flat is all blocks concatenated
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

    # Flatten stage2
    stage2_flat: list[int] = []
    for block in unique_blocks:
        stage2_flat.extend(block)

    return stage1, stage2_flat


# ============================================================================
# BINARY OUTPUT
# ============================================================================

def write_int32_binary(data: list[int], path: Path) -> None:
    """Write Int32 array as little-endian binary."""
    with open(path, 'wb') as f:
        for value in data:
            f.write(struct.pack('<i', value))
    print(f"  Wrote {path.name}: {len(data)} int32s ({len(data) * 4} bytes)")


# ============================================================================
# GENERATORS
# ============================================================================

def generate_case_data(mappings: dict[int, CaseMapping], output_dir: Path) -> None:
    """Generate binary files for case mapping tables."""

    # Build delta tables
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

    # Build and write tables
    print("  Generating uppercase tables...")
    stage1, stage2 = build_two_stage_table(upper_deltas)
    write_int32_binary(stage1, output_dir / "upper_stage1.bin")
    write_int32_binary(stage2, output_dir / "upper_stage2.bin")

    print("  Generating lowercase tables...")
    stage1, stage2 = build_two_stage_table(lower_deltas)
    write_int32_binary(stage1, output_dir / "lower_stage1.bin")
    write_int32_binary(stage2, output_dir / "lower_stage2.bin")

    print("  Generating titlecase tables...")
    stage1, stage2 = build_two_stage_table(title_deltas)
    write_int32_binary(stage1, output_dir / "title_stage1.bin")
    write_int32_binary(stage2, output_dir / "title_stage2.bin")


def generate_fold_data(folding: dict[int, list[int]], output_dir: Path) -> None:
    """Generate binary files for case folding tables."""

    # Separate simple (single char) foldings
    simple_folding: dict[int, int] = {}
    for cp, fold in folding.items():
        if len(fold) == 1 and fold[0] != cp:
            simple_folding[cp] = fold[0] - cp

    print("  Generating case folding tables...")
    stage1, stage2 = build_two_stage_table(simple_folding)
    write_int32_binary(stage1, output_dir / "fold_stage1.bin")
    write_int32_binary(stage2, output_dir / "fold_stage2.bin")


def generate_grapheme_data(properties: dict[int, str], output_dir: Path) -> None:
    """Generate binary files for grapheme break property tables."""

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

    prop_ints: dict[int, int] = {}
    for cp, prop in properties.items():
        prop_ints[cp] = PROPERTY_MAP.get(prop, 0)

    print("  Generating grapheme break property tables...")
    stage1, stage2 = build_two_stage_table(prop_ints)
    write_int32_binary(stage1, output_dir / "gbp_stage1.bin")
    write_int32_binary(stage2, output_dir / "gbp_stage2.bin")


# ============================================================================
# MAIN
# ============================================================================

def main():
    script_dir = Path(__file__).parent
    data_dir = script_dir / "data"

    # Check raw data files exist
    required_files = ["UnicodeData.txt", "CaseFolding.txt", "GraphemeBreakProperty.txt"]
    for name in required_files:
        if not (data_dir / name).exists():
            print(f"Error: {name} not found in {data_dir}")
            print("Run fetch_unicode_data.py first.")
            return 1

    print(f"Generating Unicode {UNICODE_VERSION} binary data...")
    print()

    # Parse data files
    print("Parsing UnicodeData.txt...")
    case_mappings = parse_unicode_data(data_dir / "UnicodeData.txt")
    print(f"  Found {len(case_mappings)} case mappings")

    print("Parsing CaseFolding.txt...")
    case_folding = parse_case_folding(data_dir / "CaseFolding.txt")
    print(f"  Found {len(case_folding)} case foldings")

    print("Parsing GraphemeBreakProperty.txt...")
    grapheme_props = parse_grapheme_break_property(data_dir / "GraphemeBreakProperty.txt")
    print(f"  Found {len(grapheme_props)} grapheme properties")

    print()
    print("Generating binary files...")

    # Generate binary files
    generate_case_data(case_mappings, data_dir)
    generate_fold_data(case_folding, data_dir)
    generate_grapheme_data(grapheme_props, data_dir)

    print()
    print("Done! Binary files written to data/")
    return 0


if __name__ == "__main__":
    exit(main())
