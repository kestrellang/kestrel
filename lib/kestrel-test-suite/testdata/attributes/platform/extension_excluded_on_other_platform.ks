// test: diagnostics
// stdlib: false
// host: linux

// `@platform` on an `extend` filters the whole extension.

module Test

struct S {}

@platform(.Darwin)
extend S {
    func onlyOnDarwin() -> lang.i64 { 1 }
}

func main() {
    let s = S();
    let x = s.onlyOnDarwin(); // ERROR: onlyOnDarwin
}
