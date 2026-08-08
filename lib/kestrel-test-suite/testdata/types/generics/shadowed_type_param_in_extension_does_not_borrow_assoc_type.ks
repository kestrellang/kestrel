// test: diagnostics
// stdlib: false
module Test

// Same rule across the *context* ancestor walk: `T` inside the extension body
// normally resolves to Box's parameter, which the extension's `where T: Mapper`
// bounds. A method that declares its own `[T]` shadows that, so `T.Source` must
// not pick up `Mapper.Source` from the extension's where clause.
//
// The positive counterpart is
// declarations/extensions/extension_associated_type_resolution.ks.
//
// E439 also fires: the extension's LHS `[T]` counts as an outer type parameter
// for shadowing purposes, even though the entity it binds belongs to Box.

protocol Mapper {
    type Source;
    func map(s: Source)
}

struct Box[T] { var value: T }

extend Box[T] where T: Mapper {
    func doMap[T]( // ERROR: shadows
        s: T.Source // ERROR: Source
    ) {}
}
