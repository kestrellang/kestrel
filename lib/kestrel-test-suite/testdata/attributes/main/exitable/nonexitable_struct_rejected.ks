// test: diagnostics
// stdlib: true
// executable: true

// A `@main` returning a nominal type that does NOT conform to `Exitable` is
// rejected by E616 — even with the stdlib loaded (so `Exitable` exists). The
// negative counterpart to the custom-`Exitable`-conformer execution tests.
module Main

struct NotExitable { var x: Int64 }

@main
func main() -> NotExitable { NotExitable(x: 0) } // ERROR(E616)
