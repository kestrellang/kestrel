// test: diagnostics
// stdlib: false
module Test

// `where Self.Source.Item = Self` binds every conformer: a type whose
// `Source.Item` is something else does not conform (E410), whether it
// writes the pinning protocol or a refinement of it.
protocol Source {
    type Item
}

protocol SelfSource: Source where Self.Source.Item = Self {}

protocol MoreSelfSource: SelfSource {}

struct Good: SelfSource {
    type Item = Good
}

struct Bad {}

extend Bad: SelfSource { // ERROR: 'Bad' binds 'Source.Item' to 'Good', but protocol 'SelfSource' requires 'Bad'
    type Item = Good
}

struct AlsoBad {}

extend AlsoBad: MoreSelfSource { // ERROR: 'AlsoBad' binds 'Source.Item' to 'Good', but protocol 'SelfSource' requires 'AlsoBad'
    type Item = Good
}
