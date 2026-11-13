// Decodes every PngSuite image, whole and fed one byte at a time, and
// checks the pixels against Pillow's (see make_fixtures.py). Corrupt
// files must fail.

module images_tests

import std.files.(Path, File)
import images.(RawImage, PngFormat, ImageFormat, DecodeOptions, CollectingSink, ImageSink, ImageDecoder)

/// Decodes `data` fed in `chunkSize`-byte pieces.
func decodeInChunks(data: Array[UInt8], chunkSize: Int64) -> RawImage? {
    var decoder = PngFormat().makeDecoder(DecodeOptions());
    var sink: any ImageSink = CollectingSink();
    var start: Int64 = 0;
    while start < data.count {
        var end = start + chunkSize;
        if end > data.count { end = data.count; }
        var piece: [UInt8] = [];
        for i in start..<end { piece.append(data(i)); }
        match decoder.write(piece, into: sink) {
            .Ok(_) => {},
            .Err(_) => { return .None; }
        }
        start = end;
    }
    match decoder.finish(into: sink) {
        .Ok(_) => {},
        .Err(_) => { return .None; }
    }
    guard let .Some(collected) = sink as? CollectingSink else { return .None; }
    if collected.images.count != 1 { return .None; }
    .Some(collected.images(0))
}

@main
func main() {
    let dir = directory("PNGSUITE");
    for expected in pngSuite() {
        let name = expected.name;
        guard let .Ok(data) = File.readBytes(Path(dir + name)) else {
            check("\(name) readable", false);
            continue;
        }
        match RawImage.decode(data: data) {
            .Ok(raw) => {
                if not expected.valid {
                    check("\(name) is rejected", false);
                    continue;
                }
                check("\(name) size", raw.width == expected.width and raw.height == expected.height);
                check("\(name) pixels", rgba8Checksum(raw) == expected.crc);
            },
            .Err(_) => check("\(name) decodes", not expected.valid)
        }
        if expected.valid {
            for chunk in [1, 7] {
                match decodeInChunks(data, chunk) {
                    .Some(raw) => check("\(name) in \(chunk)-byte chunks", rgba8Checksum(raw) == expected.crc),
                    .None => check("\(name) in \(chunk)-byte chunks decodes", false)
                }
            }
        }
    }
}
