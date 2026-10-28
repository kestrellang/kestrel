// test: diagnostics
// stdlib: true

// Regression (#130): a protocol-extension method with the same name as a
// stored field compiled with zero diagnostics, and `self.name` inside the
// method bound to the METHOD — infinite recursion, SIGSEGV at runtime.
// `try_resolve_through_protocol` collapsed the {field, method} candidate
// pair into "one protocol requirement" because a field's empty param shape
// trivially matches a zero-arg method. A non-callable field can never
// implement a `func` requirement, so the collision must surface as a
// genuine ambiguity instead of silently binding the recursive method.
//
// See lib/kestrel-type-infer/src/resolve.rs::try_resolve_through_protocol.

module Test

protocol Place {
    func name() -> String
    func describe()
}

struct Room {
    let name: String
}

extend Room: Place {
    func name() -> String { // ERROR: method 'name' shadows the stored field 'name' of 'Room'
        self.name // ERROR: ambiguous member 'name'
    }

    func describe() {
        println("You are in \(self.name()).") // ERROR: ambiguous member 'name'
    }
}

@main
func main() {
    let r = Room(name: "kitchen");
    r.describe();
}
