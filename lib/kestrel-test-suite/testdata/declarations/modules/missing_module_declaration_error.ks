// test: diagnostics
// stdlib: false

// A file that declares anything needs a `module` line. Without one its
// declarations sat directly under the root, where every module's lookups
// end, so its names leaked into other modules (a `struct A` here broke
// std's `extend (A, B): Equatable`).

struct A { // ERROR(E414): no `module` declaration
    var n: lang.i64
}

func f() -> lang.i64 { 1 }
