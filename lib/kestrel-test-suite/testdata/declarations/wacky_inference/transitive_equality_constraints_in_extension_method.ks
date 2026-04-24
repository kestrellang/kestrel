// test: diagnostics
// stdlib: false

module Test

protocol Processor {
    type Input;
    type Output;
    func process(i: Input) -> Output
}

struct Pipeline[P] {
    var p: P
}

extend Pipeline[P] where P: Processor {
    func transform[T](t: T, i: P.Input) -> P.Output
    where T: Processor, T.Input = P.Input, T.Output = P.Input {
        let intermediate = t.process(i);
        let twice = t.process(intermediate);
        return self.p.process(twice);
    }
}
