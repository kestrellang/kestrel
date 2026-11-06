#!/usr/bin/env python3
"""Reads `HEX <format> <input> <hex>` lines from the encode test's output
and checks that Python's zlib (the reference implementation) decompresses
each one back to the original input. Prints PASS/FAIL lines."""
import gzip, sys, zlib, importlib.util, os

spec = importlib.util.spec_from_file_location("fixtures", os.path.join(os.path.dirname(__file__), "make_fixtures.py"))
# Only the input generators are needed, not the fixture file it writes.
source = open(spec.origin).read().split("inputs = {")[0]
namespace = {}
exec(source, namespace)
inputs = {
    "empty": b"", "hello": b"hello, hello, hello!",
    "pattern": namespace["pattern"](3000), "text": namespace["text"](8000),
    "noise": namespace["noise"](1500), "zeros": bytes(100000),
}

failed = 0
for line in sys.stdin:
    if not line.startswith("HEX "):
        continue
    _, fmt, name, *rest = line.split()
    blob = bytes.fromhex(rest[0] if rest else "")
    try:
        if fmt == "raw":
            out = zlib.decompress(blob, -15)
        elif fmt == "zlib":
            out = zlib.decompress(blob)
        else:
            out = gzip.decompress(blob)
        ok = out == inputs[name]
    except Exception as error:
        ok = False
    print(("PASS" if ok else "FAIL") + f" python reads {fmt} {name}")
    failed += not ok
sys.exit(1 if failed else 0)
