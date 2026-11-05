# Deflate

DEFLATE, zlib and gzip compression for Kestrel, in pure Kestrel. Output is readable by the reference zlib, and everything zlib writes decodes here.

## Installation

```toml
[dependencies]
deflate = { path = "../../lang/deflate" }
```

## Usage

```kestrel
import deflate.(Gzip, Zlib, Deflate, CompressionLevel)

let packed = Gzip.compress(data);                     // level 6, like gzip
let smaller = Zlib.compress(data, level: .best);
let original = try Gzip.decompress(packed);          // Result[Array[UInt8], DeflateError]
```

Streaming, for data that arrives in pieces:

```kestrel
import deflate.(GzipDecoder, GzipEncoder)

var decoder = GzipDecoder();
for chunk in chunks {
    output.append(contentsOf: try decoder.write(chunk));
}
try decoder.finish();                                 // fails if the stream was cut off

var encoder = GzipEncoder(level: .fastest);
var compressed = encoder.write(part1);
compressed.append(contentsOf: encoder.write(part2));
compressed.append(contentsOf: encoder.finish());
```

## API

| Type | What it is |
|---|---|
| **Deflate**, **Zlib**, **Gzip** | One-shot `compress(data, level:)` and `decompress(data)` |
| **Inflater**, **ZlibDecoder**, **GzipDecoder** | Streaming decoders, conforming to **Decompressor** |
| **Deflater**, **ZlibEncoder**, **GzipEncoder** | Streaming encoders, conforming to **Compressor** |
| **CompressionLevel** | `0...9` like zlib, with `.none`, `.fastest`, `.standard` (6) and `.best` |
| **Crc32**, **Adler32** | Checksums, incremental `update` plus one-shot `checksum(data)` |
| **DeflateError** | What went wrong decoding: truncated input, bad header, bad checksum, ... |

- Byte inputs are `some Slice[UInt8]`, so arrays and slices both work.
- Decoders back up to the last whole symbol when a chunk ends mid-symbol, so chunks can be split anywhere.
- `GzipDecoder` reads concatenated members. `ZlibDecoder` and `GzipDecoder` reject trailing garbage; `Inflater.remainingInput` gives the bytes after a raw stream.
- Encoders pick the smallest of stored, fixed-Huffman and dynamic-Huffman for each block. Compression ratio at level 6 matches zlib's closely.
- Preset dictionaries (zlib `FDICT`) aren't supported.

## Performance

Measured with `-O 1` on a 5 MB mix of text and random bytes: decoding runs at roughly 100 MB/s and level-6 compression at roughly 25 MB/s. That is about 10× and 5× slower than C zlib. The decoder's hot loop (byte-at-a-time output with bounds checks) is the obvious first target.

## Tests

```sh
lang/deflate/tests/run.sh
```

Decodes fixtures made by Python's `zlib` and `gzip` modules (`tests/make_fixtures.py` regenerates them), round-trips every input at every level in every format, checks malformed input, and, when `python3` is available, confirms the reference zlib decodes everything this package writes.
