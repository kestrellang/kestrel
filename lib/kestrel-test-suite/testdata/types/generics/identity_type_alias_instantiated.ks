// test: diagnostics
// stdlib: false

module Test

type Identity[T] = T;
type IntAlias = Identity[lang.i64];
