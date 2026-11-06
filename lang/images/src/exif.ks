module images

// Just enough EXIF to find the orientation tag. Full EXIF parsing belongs
// in a separate package.

/// The orientation stored in raw EXIF (TIFF-structured) bytes, or
/// `.normal` when there is none or the data is malformed.
public func exifOrientation(exif: Array[UInt8]) -> Orientation {
    var start: Int64 = 0;
    // JPEG APP1 payloads start with "Exif\0\0"; PNG eXIf doesn't.
    if startsWith(exif, "Exif") { start = 6; }
    if exif.count < start + 8 { return .normal; }
    let little = exif(start) == 73 and exif(start + 1) == 73;
    let big = exif(start) == 77 and exif(start + 1) == 77;
    if not little and not big { return .normal; }
    let ifd = start + exifRead(exif, start + 4, 4, little);
    if ifd + 2 > exif.count or ifd < start { return .normal; }
    let entries = exifRead(exif, ifd, 2, little);
    for i in 0..<entries {
        let at = ifd + 2 + i * 12;
        if at + 12 > exif.count { return .normal; }
        if exifRead(exif, at, 2, little) == 0x0112 {
            return Orientation(exifValue: exifRead(exif, at + 8, 2, little));
        }
    }
    .normal
}

func exifRead(data: Array[UInt8], at: Int64, size: Int64, little: Bool) -> Int64 {
    var value: Int64 = 0;
    for i in 0..<size {
        var index = at + i;
        if little { index = at + size - 1 - i; }
        value = (value << 8) | Int64(from: data(index));
    }
    value
}
