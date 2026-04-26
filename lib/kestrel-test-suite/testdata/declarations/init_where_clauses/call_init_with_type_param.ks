// test: diagnostics
// stdlib: false

module Test

protocol Measurable {
    func measure() -> lang.i64
}

struct Metric: Measurable {
    func measure() -> lang.i64 { 42 }
}

struct Result {
    var measurement: lang.i64

    init[T](source: T) where T: Measurable {
        self.measurement = source.measure()
    }
}

func test() -> Result {
    let m = Metric();
    Result(m)
}
