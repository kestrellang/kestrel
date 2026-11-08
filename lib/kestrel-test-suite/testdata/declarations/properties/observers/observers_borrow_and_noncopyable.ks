// test: diagnostics
// stdlib: true

// An observed property can't be lent as `&mutating` (its observers could
// not run), and `didSet` only copies the old value out when it reads
// `oldValue`, so a non-Copyable property can be observed as long as it
// doesn't. A local of its own named `oldValue` doesn't count.
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

    var other: Handle {
        didSet {
            let oldValue = 2;
            self.count += oldValue;
        }
    }
}

func f() {
    var s = S(count: 1, handle: Handle(fd: 3), other: Handle(fd: 5));
    let r = &mutating s.count; // ERROR(E210): cannot take a `&mutating` borrow of observed property 'count'
    s.handle = Handle(fd: 4);
}
