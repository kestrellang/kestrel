// test: diagnostics
// stdlib: false

module Test
protocol Processor {
    func process(x: ()) -> ()
    func process(x: ()) -> () // ERROR: duplicate function signature
}
