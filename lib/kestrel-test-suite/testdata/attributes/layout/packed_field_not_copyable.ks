// test: diagnostics
// stdlib: false
//
// E911: a packed field may be misaligned, so every read copies it out; only
// bitwise-Copyable fields can be copied that way.

module Main

struct Handle: not Copyable {
    var fd: lang.i64;
}

@layout(.Packed)
struct Record {
    var tag: lang.i8;
    var handle: Handle; // ERROR: field 'handle' of packed struct 'Record' is not Copyable; packed fields are copied on every read
}

@layout(.Packed)
struct Fine {
    var tag: lang.i8;
    var value: lang.i64;
}
