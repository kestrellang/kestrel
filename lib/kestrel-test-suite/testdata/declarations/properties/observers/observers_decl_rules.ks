// test: diagnostics
// stdlib: true

// `willSet` / `didSet` only observe an instance stored `var` (E638).
module Test

import std.numeric.(Int64)

struct S {
    var raw: Int64

    let fixed: Int64 { didSet { } } // ERROR(E638): a `let` property can't have `willSet` / `didSet` observers

    var mixed: Int64 { // ERROR(E638): `willSet` / `didSet` can't be combined with `get`, `set` or `ref` accessors
        get { self.raw }
        set { self.raw = newValue; }
        didSet { }
    }

    static var shared: Int64 = 0 { willSet { } } // ERROR(E638): `willSet` / `didSet` on static and global properties are not supported yet

    subscript(i: Int64) -> Int64 { // ERROR(E638): subscripts can't have `willSet` / `didSet` observers
        get { self.raw }
        didSet { }
    }
}

var global: Int64 = 0 { didSet { } } // ERROR(E638): `willSet` / `didSet` on static and global properties are not supported yet
