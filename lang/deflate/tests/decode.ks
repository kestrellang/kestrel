// Decodes data compressed by the reference zlib (see make_fixtures.py),
// whole and in chunks of several sizes, plus malformed input.

module deflate_tests_fixtures

import deflate.(Inflater, ZlibDecoder, GzipDecoder, Decompressor, Deflate, Zlib, Gzip, DeflateError)

func decodeInChunks[D](decoder: D, data: Array[UInt8], chunkSize: Int64) -> Array[UInt8]? where D: Decompressor {
    var d = decoder;
    var output: [UInt8] = [];
    var start: Int64 = 0;
    while start < data.count {
        var end = start + chunkSize;
        if end > data.count { end = data.count; }
        match d.write(data(start..<end)) {
            .Ok(part) => output.append(contentsOf: part),
            .Err(_) => { return .None; }
        }
        start = end;
    }
    if data.count == 0 {
        match d.write(data) { .Ok(_) => {}, .Err(_) => { return .None; } }
    }
    match d.finish() { .Ok(_) => .Some(output), .Err(_) => .None }
}

func decode(format: String, data: Array[UInt8], chunkSize: Int64) -> Array[UInt8]? {
    if format == "raw" { return decodeInChunks(Inflater(), data, chunkSize); }
    if format == "zlib" { return decodeInChunks(ZlibDecoder(), data, chunkSize); }
    decodeInChunks(GzipDecoder(), data, chunkSize)
}

func expectError(name: String, result: Result[Array[UInt8], DeflateError]) {
    match result {
        .Ok(_) => check(name, false),
        .Err(_) => check(name, true)
    }
}

@main
func main() {
    for fixture in fixtures() {
        let compressed = fromHex(fixture.compressedHex);
        let expected = input(named: fixture.input);
        for chunkSize in [1000000, 1, 7, 4096] {
            let label = "\(fixture.name), chunks of \(chunkSize)";
            match decode(fixture.format, compressed, chunkSize) {
                .Some(output) => check(label, output == expected),
                .None => check(label + " (decode failed)", false)
            }
        }
    }

    // Malformed input.
    let one: UInt8 = 1;
    let good = Zlib.compress(text(8000));
    var truncated: [UInt8] = [];
    for i in 0..<(good.count - 10) { truncated.append(good(i)); }
    expectError("zlib truncated", Zlib.decompress(truncated));
    var badChecksum = good;
    badChecksum(good.count - 1) = badChecksum(good.count - 1) ^ one;
    expectError("zlib bad checksum", Zlib.decompress(badChecksum));
    var badHeader = good;
    let wrongMethod: UInt8 = 0x79;
    badHeader(0) = wrongMethod;
    expectError("zlib bad header", Zlib.decompress(badHeader));
    var trailing = good;
    trailing.append(0);
    expectError("zlib trailing byte", Zlib.decompress(trailing));

    let gzipped = Gzip.compress(text(8000));
    var badLength = gzipped;
    badLength(gzipped.count - 4) = badLength(gzipped.count - 4) ^ one;
    expectError("gzip bad length", Gzip.decompress(badLength));
    var garbage = gzipped;
    let junk: [UInt8] = [1, 2, 3];
    garbage.append(contentsOf: junk);
    expectError("gzip trailing garbage", Gzip.decompress(garbage));

    let reserved: [UInt8] = [0x07];
    expectError("reserved block type", Deflate.decompress(reserved));
    let storedMismatch: [UInt8] = [0x01, 0x05, 0x00, 0x00, 0x00];
    expectError("stored length mismatch", Deflate.decompress(storedMismatch));
    // A fixed block whose first symbol is a match with nothing before it.
    let earlyMatch: [UInt8] = [0x03, 0x02];
    expectError("distance before start", Deflate.decompress(earlyMatch));
    let nothing: [UInt8] = [];
    expectError("empty input", Deflate.decompress(nothing));
}
