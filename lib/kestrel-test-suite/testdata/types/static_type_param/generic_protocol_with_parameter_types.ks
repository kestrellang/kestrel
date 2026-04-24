// test: diagnostics
// stdlib: false

module Test

protocol Processor[Input] {
    func process(input: Input) -> lang.i64
}
func runProcessor[P](p: P, input: lang.str) -> lang.i64 where P: Processor[lang.str] {
    p.process(input)
}
