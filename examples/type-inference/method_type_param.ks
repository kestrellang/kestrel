// Minimal reproduction of type parameter identity issue
// When a method with its own type parameter is called, the inferred
// type parameter should unify with the caller's type parameter

module examples.type_inference

public enum Optional[T] {
    case Some(T)
    case None

    public func map[U](transform: (T) -> U) -> Optional[U] {
        match self {
            .Some(value) => .Some(transform(value)),
            .None => .None
        }
    }
}

public protocol Iterator {
    type Item

    mutating func next() -> Optional[Item]
}

// This struct has type parameter U and uses it in a method call
public struct MapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    var inner: I
    var transform: (I.Item) -> U

    public mutating func next() -> Optional[U] {
        // self.inner.next() returns Optional[I.Item]
        // self.transform has type (I.Item) -> U (where U is MapIterator's type param)
        // When we call .map(self.transform), it should infer that:
        //   Optional.map's U = MapIterator's U
        // But instead we get: expected U, found U (different SymbolIds)
        self.inner.next().map(self.transform)
    }
}
