// test: diagnostics
// stdlib: true

// Provider exclusivity (stage 1.5): at most one READ provider (`get` XOR
// `ref`) and one WRITE provider (`set` XOR `mutating ref`) per member.
module Test

import std.memory.(Pointer)
import std.numeric.(Int64)

struct BothReads {
    var p: Pointer[Int64]
    subscript(at index: Int64) -> Int64 { // ERROR(E619)
        get { self.p.offset(by: index).value }
        ref { self.p.offset(by: index).value }
    }
}

struct BothWrites {
    var p: Pointer[Int64]
    subscript(at index: Int64) -> Int64 { // ERROR(E620)
        ref { self.p.offset(by: index).value }
        set { self.p.offset(by: index).write(newValue); }
        mutating ref { self.p.offset(by: index).mutatingValue }
    }
}

struct BothOnProperty {
    var p: Pointer[Int64]
    var value: Int64 { // ERROR(E619)
        get { self.p.value }
        ref { self.p.value }
    }
}
