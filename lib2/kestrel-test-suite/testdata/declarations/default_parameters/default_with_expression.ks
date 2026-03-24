// test: diagnostics
// stdlib: false

module Main

func getDefault() -> lang.i64 { 42 }

func process(value: lang.i64 = getDefault()) { }
