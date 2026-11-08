// test: diagnostics
// stdlib: false

module Test

@deprecated
@unavailable // ERROR: '@unavailable' cannot be combined with '@deprecated'
func a() {}

@unavailable
@main // ERROR: '@main' cannot be combined with '@unavailable': the entry point must be usable
func b() {}
