// test: diagnostics
// stdlib: false

module Test

protocol Base {}
protocol Left: Base {}
protocol Right: Base {}
protocol Diamond: Left, Right {}
