// test: diagnostics
// stdlib: true

// A `{ ref }` / `{ mutating ref }` requirement lends the place itself, so a
// computed `get` / `set` property can't meet it (E405). A bodyless `ref`
// only declares a requirement: outside a protocol it is E640, and a
// protocol extension can't add a requirement (E621).
module Test

import std.numeric.(Int64)

protocol Holder {
    var item: Int64 { ref mutating ref }
}

protocol Reader {
    var item: Int64 { ref }
}

struct GetSet: Holder { // ERROR(E405): must lend a reference (`ref`)
    var v: Int64
    var item: Int64 {
        get { self.v }
        set { self.v = newValue }
    }
}

struct RefOnly: Holder { // ERROR(E405): must lend a reference (`mutating ref`)
    var v: Int64
    var item: Int64 {
        ref { self.v }
    }
}

struct Constant: Holder { // ERROR(E460)
    let item: Int64
}

struct ReadOnlyOk: Reader {
    let item: Int64
}

struct Bodyless {
    var item: Int64 { ref } // ERROR(E640)
}

extend Reader {
    var other: Int64 { ref } // ERROR(E621)
}
