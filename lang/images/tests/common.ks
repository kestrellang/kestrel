// Helpers shared by the image test programs.

module images_tests

import std.system.(Environment)
import deflate.(Crc32)
import images.(RawImage, Rgba16, ConversionOptions, Image)

func check(name: String, ok: Bool) {
    if ok { print("PASS \(name)\n"); } else { print("FAIL \(name)\n"); }
}

/// The directory named by an environment variable, with a trailing slash.
func directory(variable: String) -> String {
    match Environment.variable(variable) {
        .Some(dir) => "\(dir)/",
        .None => "./"
    }
}

/// The pixels as 8-bit RGBA (high byte of 16-bit samples), summarized as
/// CRC-32: the same reduction make_fixtures.py applies with Pillow.
func rgba8Checksum(raw: RawImage) -> Int64 {
    match raw.converted[Rgba16](options: ConversionOptions()) {
        .Ok(image) => {
            var bytes: [UInt8] = [];
            for y in 0..<image.height {
                for x in 0..<image.width {
                    let p = image(x, y);
                    bytes.append(UInt8(truncating: p.r >> 8));
                    bytes.append(UInt8(truncating: p.g >> 8));
                    bytes.append(UInt8(truncating: p.b >> 8));
                    bytes.append(UInt8(truncating: p.a >> 8));
                }
            }
            Int64(from: Crc32.checksum(bytes))
        },
        .Err(_) => -1
    }
}
