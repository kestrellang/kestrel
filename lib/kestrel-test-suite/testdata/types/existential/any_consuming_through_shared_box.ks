// test: diagnostics
// stdlib: true
//
// E148: consuming requirements need a box that is `ConsumingIndirection`.
// RcBox (the default) and CowBox share their payload (design §3, §6).

module Test

import std.numeric.(Int64)
import std.core.(Cloneable)
import std.memory.(CowBox)

protocol Closeable: Cloneable {
    consuming func close() -> Int64
}

struct File: Cloneable {
    var fd: Int64;
    public func clone() -> File { File(fd: self.fd) }
}
extend File: Closeable {
    public consuming func close() -> Int64 { self.fd }
}

func test() {
    let a: any Closeable = File(fd: 3);
    let x = a.close(); // ERROR: cannot call consuming method 'close' on 'any Closeable': 'RcBox' shares its payload, so it cannot give it up; use 'any[UniqueBox] Closeable'
    let b: any[CowBox] Closeable = File(fd: 4);
    let y = b.close(); // ERROR: cannot call consuming method 'close' on 'any[CowBox] Closeable': 'CowBox' shares its payload, so it cannot give it up; use 'any[UniqueBox] Closeable'
}
