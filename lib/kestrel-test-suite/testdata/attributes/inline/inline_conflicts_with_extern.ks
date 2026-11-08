// test: diagnostics
// stdlib: false

module Test

@extern(.C)
@inline // ERROR: '@inline' cannot be combined with '@extern': an extern function has no body to inline
func external()
