// test: diagnostics
// stdlib: false

// `@platform(.Linux, .Darwin)` means *any of*: the declaration is kept on
// both hosts, so the call resolves.

module Test

@platform(.Linux, .Darwin)
func everywhere() -> lang.i64 { 42 }

func main() {
    let x = everywhere();
}
