// test: diagnostics
// stdlib: false

module Test

struct S {}

@platform(.Linux, .Darwin)
extend S {
    func everywhere() -> lang.i64 { 1 }
}

func main() {
    let s = S();
    let x = s.everywhere();
}
