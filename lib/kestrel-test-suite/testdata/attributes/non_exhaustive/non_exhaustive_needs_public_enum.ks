// test: diagnostics
// stdlib: false

module Test

@nonExhaustive // ERROR: '@nonExhaustive' has no effect: 'Mode' is not visible outside its module
enum Mode {
    case Fast
}

@nonExhaustive // ERROR: '@nonExhaustive' cannot be applied to a struct
public struct Config {}
