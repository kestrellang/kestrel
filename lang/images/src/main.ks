/// Image decoding, encoding and processing in pure Kestrel.
///
/// PNG, QOI and BMP are built in; other packages add formats through a
/// `Formats` registry. Most code uses the typed `Image[P]`; `RawImage`
/// holds exactly what a file had, and the streaming `ImageDecoder` /
/// `ImageSink` layer handles huge images and data that arrives in pieces.
///
/// # Examples
///
/// ```
/// import images.(Image, Rgba8)
///
/// let photo = try Image[Rgba8].load(path: "cat.png");
/// let thumb = photo.resized(width: 256, height: 256 * photo.height / photo.width);
/// try thumb.save(path: "thumb.png");
/// ```
module images
