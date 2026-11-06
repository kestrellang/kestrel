// test: diagnostics
// stdlib: false

module Test

// Non-generic types (and their extensions) keep stored statics.

struct Plain {
    static var count: lang.i64 = 0;
}

extend Plain {
    static var more: lang.i64 = 1;
}
