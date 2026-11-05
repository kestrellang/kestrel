// test: execution
// stdlib: true
// expect-exit: 0

// Two unrelated protocols both require `read()`. E463's own advice is to
// give each protocol its own implementation in an `extend Type: Protocol`
// block. That is what this does, and it should compile, with each call
// through a protocol reaching that protocol's implementation.
//
// Known gap: the compiler reports the two extension methods as duplicates
// (E412) and still as ambiguous (E463). std.system hit this while std.io
// and std.streams coexisted: its pipes couldn't conform to both libraries'
// `Readable`.

module Test

protocol OldSource {
    mutating func read() -> Int64
}

protocol NewSource {
    mutating func read() -> Int64
}

struct Pipe {
    var n: Int64
}

extend Pipe: OldSource {
    mutating func read() -> Int64 { 1 }
}

extend Pipe: NewSource {
    mutating func read() -> Int64 { 2 }
}

func readOld[S](mutating source: S) -> Int64 where S: OldSource { source.read() }
func readNew[S](mutating source: S) -> Int64 where S: NewSource { source.read() }

@main
func main() -> lang.i64 {
    var p = Pipe(n: 0);
    if readOld(p) != 1 { return 1 }
    if readNew(p) != 2 { return 2 }
    0
}
