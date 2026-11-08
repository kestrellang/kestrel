// test: diagnostics
// stdlib: true

// A throwing place may not exist, so like a failable one it has no `set`
// (E637) and can't be assigned (E214).
module Test

import std.numeric.(Int64)

enum Bad {
    case Nope
}

struct WithSet {
    var x: Int64 throws Bad { // ERROR(E637): a throwing place cannot have a `set` accessor
        get { throw Bad.Nope; }
        set { }
    }
}

struct Reader {
    var raw: Int64
    var x: Int64 throws Bad {
        get { .Ok(self.raw) }
    }
}

func f() {
    var r = Reader(raw: 1);
    r.x = .Ok(2); // ERROR(E214): cannot assign to throwing place 'x'
}
