// Copying one stream into another.

module std.streams

import std.numeric.(Int64, UInt8)
import std.result.(Result)
import std.collections.(Array)
import std.io.error.(IoError)

/// Reads `reader` to its end and writes everything to `writer`. Returns
/// the number of bytes copied. Doesn't flush `writer`.
///
/// # Examples
///
/// ```
/// var out = Buffer();
/// try copy(from: Cursor(text: "hi"), to: out);
/// ```
internal func copy[R, W](mutating from reader: R, mutating to writer: W) -> Result[Int64, IoError]
    where R: Readable, R: not Copyable, R: not Static, W: Writable, W: not Copyable, W: not Static {
    var chunk = Array[UInt8](repeating: 0, count: 8192);
    var total: Int64 = 0;
    loop {
        let n = match reader.read(into: chunk.asMutableSlice()) {
            .Ok(n) => n,
            .Err(e) => {
                if isInterrupted(e) { continue }
                return .Err(e)
            }
        };
        if n == 0 {
            break
        }
        try writer.writeAll(from: chunk(0..<n));
        total = total + n
    }
    .Ok(total)
}
