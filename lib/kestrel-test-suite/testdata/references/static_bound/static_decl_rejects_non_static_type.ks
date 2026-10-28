// test: diagnostics
// stdlib: true

// Globals live for the whole program — their types must be Static.
// Module-level value declarations and `static` members are both checked.

module Test

struct Handle: not Static {
    var id: Int64
}

let GOOD: Int64 = 7;

var BAD: Handle = Handle(id: 1); // ERROR(E505)

struct Registry {
    static var shared: Handle = Handle(id: 2) // ERROR(E505)

    // Computed statics store nothing — not checked.
    static var version: Int64 { 3 }
}
