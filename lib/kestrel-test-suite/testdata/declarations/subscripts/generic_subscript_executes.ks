// test: diagnostics

module Test

import std.text.Formattable
import std.text.String

struct Formatter {
    public init() {}

    public subscript[F](value: F) -> String where F: Formattable {
        get {
            value.formatted()
        }
    }
}

func main() -> lang.i64 {
    let f = Formatter();
    let s = f(42);
    // Return byte count of "42" which is 2
    s.byteCount.raw
}
