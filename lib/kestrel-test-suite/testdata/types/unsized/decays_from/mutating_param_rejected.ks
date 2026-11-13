// test: diagnostics
// stdlib: true

// A `mutating` parameter never converts a slice: the writes would land in a
// temporary copy and be lost.

module Test

func shout(mutating x: String) { x.append("!") }

@main
func main() {
    let s = "  hi  ";
    shout(s.trimmed()); // ERROR
}
