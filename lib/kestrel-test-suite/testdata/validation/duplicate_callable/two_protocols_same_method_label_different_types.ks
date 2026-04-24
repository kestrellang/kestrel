// test: diagnostics
// stdlib: false

module Test
protocol Alpha {
    func process(value: ()) -> ()
}

protocol Beta {
    func process(value: ((), ())) -> ()
}

struct Handler: Alpha, Beta {
    func process(value: ()) -> () { () }
    func process(value: ((), ())) -> () { () }
}
