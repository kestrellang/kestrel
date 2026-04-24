// test: diagnostics
// stdlib: false

module Test

protocol Display { }
protocol Debug { }
struct Logger[T] where T: Display and Debug { }
