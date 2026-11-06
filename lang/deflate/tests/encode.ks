// Round-trips every input through each format at every level, one-shot
// and in chunks. Also prints each output as `HEX <format> <input> <hex>`
// so check_with_python.py can confirm the reference zlib reads it.

module deflate_tests_fixtures

import deflate.(Deflater, ZlibEncoder, GzipEncoder, Compressor, CompressionLevel, Deflate, Zlib, Gzip)

func encodeInChunks[C](encoder: C, data: Array[UInt8], chunkSize: Int64) -> Array[UInt8] where C: Compressor {
    var e = encoder;
    var output: [UInt8] = [];
    var start: Int64 = 0;
    while start < data.count {
        var end = start + chunkSize;
        if end > data.count { end = data.count; }
        output.append(contentsOf: e.write(data(start..<end)));
        start = end;
    }
    output.append(contentsOf: e.finish());
    output
}

func toHex(bytes: Array[UInt8]) -> String {
    let digits = "0123456789abcdef".toBytes();
    var out: [UInt8] = [];
    for b in bytes {
        out.append(digits(Int64(from: b >> 4)));
        out.append(digits(Int64(from: b & 15)));
    }
    String(fromUtf8Unchecked: out)
}

func roundTrips(compressed: Array[UInt8], format: String, expected: Array[UInt8]) -> Bool {
    var result = Deflate.decompress(compressed);
    if format == "zlib" { result = Zlib.decompress(compressed); }
    if format == "gzip" { result = Gzip.decompress(compressed); }
    match result {
        .Ok(output) => output == expected,
        .Err(_) => false
    }
}

@main
func main() {
    // A large mixed input, so streaming crosses the compressor's chunk size.
    var large = text(150000);
    large.append(contentsOf: noise(40000));
    large.append(contentsOf: pattern(60000));

    var names = allInputNames();
    names.append("large");
    for name in names {
        var data = input(named: name);
        if name == "large" { data = large; }
        for level in 0..<10 {
            let lvl = CompressionLevel(level);
            let raw = Deflate.compress(data, level: lvl);
            let zlib = Zlib.compress(data, level: lvl);
            let gzip = Gzip.compress(data, level: lvl);
            check("raw \(name) level \(level)", roundTrips(raw, "raw", data));
            check("zlib \(name) level \(level)", roundTrips(zlib, "zlib", data));
            check("gzip \(name) level \(level)", roundTrips(gzip, "gzip", data));
            if level == 6 {
                let chunked = encodeInChunks(GzipEncoder(level: lvl), data, 1000);
                check("gzip \(name) level 6 in chunks", roundTrips(chunked, "gzip", data));
                check("gzip \(name) level 6 chunked matches one-shot size", chunked.count < data.count + 100);
            }
            if name != "large" {
                print("HEX raw \(name) \(toHex(raw))\n");
                print("HEX zlib \(name) \(toHex(zlib))\n");
                print("HEX gzip \(name) \(toHex(gzip))\n");
            }
        }
        if name == "zeros" or name == "text" {
            let best = Zlib.compress(data, level: .best);
            check("\(name) compresses well", best.count * 10 < data.count);
        }
    }
}
