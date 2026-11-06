#!/usr/bin/env python3
"""Checks that Pillow reads every file features.ks saved with the pixels
features.ks wrote. Prints one PASS/FAIL line per file.

Usage: python3 check_with_python.py <directory>   (needs Pillow and numpy)
"""
import os
import sys

import numpy as np
from PIL import Image

WIDTH, HEIGHT = 37, 23
# Channels and bytes per sample for each layout name in a file name.
LAYOUTS = {
    "gray8": (1, 1), "grayAlpha8": (2, 1), "rgb8": (3, 1), "rgba8": (4, 1),
    "gray16": (1, 2), "grayAlpha16": (2, 2), "rgb16": (3, 2), "rgba16": (4, 2),
}


def pattern(channels, sample_bytes):
    """The bytes patternRaw writes, as samples (16-bit ones little-endian)."""
    bpp = channels * sample_bytes
    y, x, k = np.meshgrid(np.arange(HEIGHT), np.arange(WIDTH), np.arange(bpp), indexing="ij")
    raw = ((x * 7 + y * 13 + k * 51 + x * y) % 256).astype(np.uint32)
    if sample_bytes == 2:
        raw = raw[:, :, 0::2] | (raw[:, :, 1::2] << 8)
    return raw


def check(path):
    stem = os.path.basename(path).split(".")[0].split("-")[0]
    channels, sample_bytes = LAYOUTS[stem]
    want = pattern(channels, sample_bytes)
    image = Image.open(path)
    if image.size != (WIDTH, HEIGHT):
        return False
    if sample_bytes == 2 and channels == 1:
        got = np.array(image).astype(np.uint32)
        return np.array_equal(got, want[:, :, 0])
    if sample_bytes == 2:
        # Pillow keeps only the high byte of 16-bit color samples.
        want = want >> 8
    got = np.array(image.convert("RGBA")).astype(np.uint32)
    if channels <= 2:
        gray = want[:, :, 0]
        alpha = want[:, :, 1] if channels == 2 else np.full(gray.shape, 255)
        return np.array_equal(got[:, :, 0], gray) and np.array_equal(got[:, :, 3], alpha)
    alpha = want[:, :, 3] if channels == 4 else np.full(want.shape[:2], 255)
    return np.array_equal(got[:, :, :3], want[:, :, :3]) and np.array_equal(got[:, :, 3], alpha)


def main():
    directory = sys.argv[1]
    for name in sorted(os.listdir(directory)):
        ok = False
        try:
            ok = check(os.path.join(directory, name))
        except Exception as error:  # report any reader failure as a FAIL
            print(f"FAIL pillow reads {name}: {error}")
            continue
        print(f"{'PASS' if ok else 'FAIL'} pillow reads {name}")


if __name__ == "__main__":
    main()
