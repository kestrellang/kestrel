// test: diagnostics
// stdlib: true

// An observed property can't be lent as `&mutating` (its observers could
// not run), and `didSet` only copies the old value out when it names
// `oldValue`, so a non-Copyable property can be observed as long as it
// doesn't.
module Test

import std.numeric.(Int64)

struct Handle: not Copyable {
    var fd: Int64
}

struct S {
    var count: Int64 {
        didSet { }
    }

    var handle: Handle {
        didSet { self.count += 1; }
    }
}

func f() {
    var s = S(count: 1, handle: Handle(fd: 3));
    let r = &mutating s.count; // ERROR(E210): cannot take a `&mutating` borrow of a get/set member
    s.handle = Handle(fd: 4);
}
