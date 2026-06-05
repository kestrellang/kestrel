// test: execution
// stdlib: false
// expect-exit: 42

// An inherent method added to the unit type `()` resolves and runs.
module Main

extend () { func answer() -> lang.i64 { 42 } }

@main
func main() -> lang.i64 {
    let u = ();
    u.answer()
}
