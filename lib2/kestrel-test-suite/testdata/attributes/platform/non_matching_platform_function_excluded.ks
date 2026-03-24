// test: diagnostics
// stdlib: false

module Test

@platform(.linux)
func excluded() -> lang.i64 { 42 }

func main() {
    let x = excluded(); // ERROR: excluded
}
