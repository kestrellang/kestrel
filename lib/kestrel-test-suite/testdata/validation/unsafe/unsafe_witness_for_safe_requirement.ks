// test: diagnostics
// stdlib: false
//
// E642: generic code calls a protocol requirement without `unsafe`, so a
// witness may only be unsafe when its requirement is.

module Test

protocol Reader {
    func fetch() -> lang.i64
    unsafe func getRaw() -> lang.i64
}

struct R: Reader {
    unsafe func fetch() -> lang.i64 { 1 } // ERROR: 'fetch' is unsafe, but the requirement it satisfies in protocol 'Reader' is not
    unsafe func getRaw() -> lang.i64 { 2 }
}
