// Simplest reproduction: method on generic type with own type parameter
// Issue: T in Optional.map's parameter isn't being substituted

module examples.simple_method_subst

public struct Box[T] {
    var value: T

    // Method has its own type parameter U
    public func map[U](transform: (T) -> U) -> Box[U] {
        Box[U](value: transform(self.value))
    }
}

public struct Wrapper[A] {
    var box: Box[A]
    var transform: (A) -> A

    // Here we call box.map with self.transform
    // box has type Box[A], so T should be substituted with A
    // Expected parameter type: (A) -> U (after T→A substitution)
    // But we're getting: (T) -> U (no substitution)
    public func apply() -> Box[A] {
        self.box.map(self.transform)
    }
}
