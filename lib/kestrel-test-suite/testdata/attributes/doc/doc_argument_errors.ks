// test: diagnostics
// stdlib: false

module Test

@doc // ERROR: '@doc' needs a doc option: '@doc(.Hidden)'
public func a() {}

@doc(.Inline) // ERROR: '.Inline' is not a doc option; expected '.Hidden'
public func b() {}
