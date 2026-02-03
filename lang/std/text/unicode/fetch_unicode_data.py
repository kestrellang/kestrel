#!/usr/bin/env python3
"""
Fetches Unicode Character Database files for Kestrel's Unicode support.

Downloads from Unicode 15.1.0:
- UnicodeData.txt - case mappings, general category
- SpecialCasing.txt - 1-to-many case mappings
- CaseFolding.txt - case folding for case-insensitive comparison
- GraphemeBreakProperty.txt - grapheme cluster boundaries (UAX #29)

Usage:
    python fetch_unicode_data.py

Files are saved to the data/ subdirectory.
"""

import os
import urllib.request
from pathlib import Path

UNICODE_VERSION = "15.1.0"
BASE_URL = f"https://www.unicode.org/Public/{UNICODE_VERSION}/ucd"

FILES = {
    "UnicodeData.txt": f"{BASE_URL}/UnicodeData.txt",
    "SpecialCasing.txt": f"{BASE_URL}/SpecialCasing.txt",
    "CaseFolding.txt": f"{BASE_URL}/CaseFolding.txt",
    "GraphemeBreakProperty.txt": f"{BASE_URL}/auxiliary/GraphemeBreakProperty.txt",
}

def fetch_file(name: str, url: str, output_dir: Path) -> None:
    """Download a single file."""
    output_path = output_dir / name

    if output_path.exists():
        print(f"  {name}: already exists, skipping")
        return

    print(f"  {name}: downloading...")
    try:
        # Add User-Agent header to avoid 403 errors
        request = urllib.request.Request(
            url,
            headers={"User-Agent": "Kestrel-Unicode-Fetcher/1.0"}
        )
        with urllib.request.urlopen(request) as response:
            output_path.write_bytes(response.read())
        print(f"  {name}: done ({output_path.stat().st_size:,} bytes)")
    except Exception as e:
        print(f"  {name}: FAILED - {e}")
        raise

def main():
    script_dir = Path(__file__).parent
    data_dir = script_dir / "data"

    # Create data directory if needed
    data_dir.mkdir(exist_ok=True)

    # Create .gitignore in data directory
    gitignore_path = data_dir / ".gitignore"
    if not gitignore_path.exists():
        gitignore_path.write_text("# Downloaded UCD files - regenerate with fetch_unicode_data.py\n*\n!.gitignore\n")

    print(f"Fetching Unicode {UNICODE_VERSION} data files...")
    print(f"Output directory: {data_dir}")
    print()

    for name, url in FILES.items():
        fetch_file(name, url, data_dir)

    print()
    print("Done! Run generate_tables.py to generate Kestrel source files.")

if __name__ == "__main__":
    main()
