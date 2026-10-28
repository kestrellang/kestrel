// test: diagnostics
// stdlib: false
module Test

// A method's own type parameter shadows the enclosing type's parameter of the
// same name. The inner `T` carries no bounds, so `T.Source` must NOT be
// resolved against `Holder`'s `where T: Mapper` — where-clause subjects are
// matched by entity identity, not by name string.

protocol Mapper {
    type Source;
    func map(s: Source)
}

struct Holder[T] where T: Mapper {
    var value: T

    func f[T]( // ERROR: shadows
        s: T.Source // ERROR: Source
    ) {}
}
