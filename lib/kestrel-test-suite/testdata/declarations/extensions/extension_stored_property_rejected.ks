// test: diagnostics

// An extension cannot add instance storage to its target; the type's layout
// is fixed by its declaration. Computed and static properties are fine.

module Test

struct P {
    var x: Int64;
}

extend P {
    var y: Int64 = 0; // ERROR: extensions cannot add stored properties
    var doubled: Int64 { self.x * 2 }
    static let origin: Int64 = 0;
}
