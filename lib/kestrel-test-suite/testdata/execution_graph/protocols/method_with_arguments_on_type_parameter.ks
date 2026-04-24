// test: diagnostics
// stdlib: false

module Test

protocol Processor {
    func process(x: lang.i64, y: lang.i64) -> lang.i64
}

func run[T](proc: T, a: lang.i64, b: lang.i64) -> lang.i64 where T: Processor {
    return proc.process(a, b)
}
