# Images

Image decoding, encoding and processing for Kestrel, in pure Kestrel. PNG, QOI and BMP are built in; other packages add formats (JPEG, GIF, WebP) through a registry.

## Installation

```toml
[dependencies]
images = { path = "../../lang/images" }
```

`images` depends on [`deflate`](../deflate) for PNG's compression.

## Usage

```kestrel
import images.(Image, Rgba8, Rect)

var photo = try Image[Rgba8].load(path: "cat.png");    // upright, converted to 8-bit RGBA
photo(10, 20) = Rgba8(r: 255, g: 0, b: 0, a: 255);
let thumb = photo.resized(width: 256, height: 192);      // Lanczos3 in linear light
let face = photo.cropped(rect: Rect(x: 40, y: 30, width: 100, height: 100));
try thumb.save(path: "thumb.png");                       // format from the extension
let bytes = try photo.encoded(format: "qoi");
```

`RawImage` keeps exactly what the file had (16-bit samples, HDR color, metadata) for converters and lossless re-encoding:

```kestrel
import images.(RawImage, RgbaF32, ConversionOptions)

let raw = try RawImage.load(path: "scan.png");
print(raw.layout.description);                           // "rgb16"
print(raw.color.isHighDynamicRange);
let linear = try raw.converted[RgbaF32](options: ConversionOptions());
```

Streaming: bytes go in as they arrive and rows come out to a sink, so a huge image or a network body never has to sit in memory at once.

```kestrel
import images.(PngFormat, ImageFormat, DecodeOptions, ImageSink, CollectingSink)

var decoder = PngFormat().makeDecoder(DecodeOptions());
var sink: any ImageSink = MyThumbnailSink(width: 256);
for chunk in body { try decoder.write(chunk, into: sink); }
try decoder.finish(into: sink);
```

Adding a format:

```kestrel
var formats = Formats.standard;
formats.add(WebpFormat());                               // any `ImageFormat`
let image = try Image[Rgba8].load(path: "cat.webp", formats: formats);
```

## What's supported

| | Decode | Encode |
|---|---|---|
| PNG | Every color type and bit depth, Adam7 interlacing, palettes and tRNS, cICP, iCCP, gAMA, pHYs, text, eXIf; unknown chunks kept. APNG files give their default image. | 8 and 16-bit gray, gray+alpha, RGB, RGBA; per-row filter choice; metadata and cICP written back |
| QOI | RGB and RGBA | RGB and RGBA |
| BMP | 1, 4 and 8-bit palettes; 16, 24 and 32-bit, with bit-field masks and alpha | 24-bit RGB, 32-bit RGBA |

- **Pixel types:** `Gray8`, `GrayAlpha8`, `Gray16`, `Rgb8`, `Rgba8`, `Bgra8`, `Rgba16`, `RgbaF16`, `RgbaF32`. Images store plain bytes, so `Image[P]` and `RawImage` convert without copying.
- **Color:** CICP code points (sRGB, linear, Display P3, BT.2020, PQ, HLG). Conversion handles transfer curves, primaries and premultiplied alpha. HDR loaded into an integer type is tone mapped with BT.2390 (1000-nit content to SDR white at 203 nits); float types get linear light with values above 1. ICC profiles are kept but not applied.
- **Operations:** crop, zero-copy views, flips, 90° rotations, resize (nearest, bilinear, Lanczos3, with pixel centers at +0.5), alpha blending, fill. Resize and blend work in linear light with premultiplied alpha.
- **Orientation:** EXIF orientation is read and recorded on `RawImage`; `Image[P]` helpers apply it.
- **Safety:** `DecodeOptions.maxPixels` (default 100 million) and `maxMetadataBytes` guard against decompression bombs.

Not yet: JPEG, GIF, WebP and TIFF (separate packages), APNG frames, planar YCbCr conversion, a streaming encoder, dithering.

## Performance

On a 2000 × 2000 RGBA PNG (8.4 MB), built with `-O 1`: decode about 0.7 s, PNG encode about 3 s, QOI encode about 0.1 s, Lanczos resize to 500 × 500 about 0.3 s. Pillow (C) decodes the same file in about 0.12 s and encodes it in about 2.2 s.

## Tests

```sh
lang/images/tests/run.sh            # needs target/release/kestrel
```

- `pngsuite.ks` decodes all 175 [PngSuite](http://www.schaik.com/pngsuite/) images whole and fed 1 and 7 bytes at a time, and checks pixels against Pillow (`make_fixtures.py` regenerates the expected values). The 14 corrupt files must fail.
- `features.ks` covers round trips through every encoder, color and HDR conversion, metadata, orientation, operations, limits and malformed input.
- `check_with_python.py` checks that Pillow reads every file the package writes.
