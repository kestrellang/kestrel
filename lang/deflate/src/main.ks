/// DEFLATE (RFC 1951), zlib (RFC 1950) and gzip (RFC 1952) in pure Kestrel.
///
/// One-shot helpers cover most uses. The streaming types take input in
/// chunks of any size, for files and network bodies that shouldn't sit in
/// memory at once.
///
/// # Examples
///
/// ```
/// import deflate.(Gzip, GzipDecoder)
///
/// let packed = Gzip.compress(data, level: .best);
/// let unpacked = try Gzip.decompress(packed);
///
/// var decoder = GzipDecoder();
/// for chunk in body {
///     output.append(contentsOf: try decoder.write(chunk));
/// }
/// try decoder.finish();
/// ```
module deflate
