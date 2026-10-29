// test: execution
// stdlib: true
// expect-exit: 0

// A plain init also satisfies a THROWING requirement (`.Ok` wrapping), and a
// generic conformer's plain init satisfies a failable one: the witness is an
// adapter that runs the init and returns the requirement's success case.

module Test

import std.numeric.Int64
import std.text.String

enum ParseError { case Bad }

protocol Decodable {
    init(from source: Int64) throws ParseError
}

protocol Parseable {
    init(from source: Int64)?
}

struct Always: Decodable {
    var value: Int64
    init(from source: Int64) { self.value = source }
}

struct Box[T]: Parseable {
    var value: Int64
    var tag: T?
    init(from source: Int64) { self.value = source; self.tag = null; }
}

func decode[T](from source: Int64) -> T throws ParseError where T: Decodable { T(from: source) }
func parse[T](from source: Int64) -> T? where T: Parseable { T(from: source) }

@main
func main() -> lang.i64 {
    let decoded: Always throws ParseError = decode[Always](from: 11);
    match decoded {
        .Ok(a) => { if a.value != 11 { return 1 } },
        _ => { return 2 }
    }
    let parsed: Box[String]? = parse[Box[String]](from: 5);
    match parsed {
        .Some(b) => { if b.value != 5 { return 3 } },
        _ => { return 4 }
    }
    0
}
