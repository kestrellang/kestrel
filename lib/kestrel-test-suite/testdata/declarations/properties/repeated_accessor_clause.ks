// test: diagnostics
// stdlib: true

// An accessor clause written twice was silently ignored: only the first
// ran. Read clauses report E619, write clauses and observers E620.
module Test

struct S {
    var n: Int64

    var a: Int64 { // ERROR(E619): `get` is written more than once on this member
        get { 1 }
        get { 2 }
    }

    var b: Int64 { // ERROR(E619): `modifying get` is written more than once on this member
        modifying get { self.n }
        modifying get { self.n }
    }

    var c: Int64 { // ERROR(E620): `set` is written more than once on this member
        get { self.n }
        set { self.n = newValue; }
        set { self.n = 0; }
    }

    var d: Int64 = 0 { // ERROR(E620): `didSet` is written more than once on this member
        didSet { }
        didSet { }
    }
}
