// test: diagnostics
// stdlib: true

module Main

// `for` desugars to Sugar{ForLoop} wrapping the Block/Loop/Match chain.
// The dead-code analyzer must see through the Sugar wrapper, or E002 is
// structurally blind inside every `for` body.
func test() -> std.numeric.Int64 {
    for i in std.core.Range[std.numeric.Int64](0, 5) {
        return i;
        let x: std.numeric.Int64 = 1; // WARN: unreachable
    }
    0
}
