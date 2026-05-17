// test: diagnostics
// stdlib: false

module Test

protocol Counter {
    mutating func increment()
}

func bump[T](a: T) where T: Counter {
    var x = a;
    x.increment()
}
