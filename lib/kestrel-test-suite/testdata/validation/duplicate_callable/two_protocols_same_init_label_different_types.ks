// test: diagnostics
// stdlib: false

module Test
protocol Alpha {
    init(value: ())
}

protocol Beta {
    init(value: ((), ()))
}

struct Widget: Alpha, Beta {
    let x: ()

    init(value: ()) { self.x = value }
    init(value: ((), ())) { self.x = () }
}
