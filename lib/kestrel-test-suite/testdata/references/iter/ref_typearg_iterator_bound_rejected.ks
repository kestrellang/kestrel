// test: diagnostics
// stdlib: true

// Stage 2d gate (must stay): a BARE ref as a type argument satisfies
// only Copyable — instantiating a bounded generic at `&Int64` is a
// clean DoesNotConform (the implicit Static bound fires first: a
// reference is never Static), never a mono witness ICE.
module Test

struct Wants[I] where I: Iterator {
    var marker: Int64
}

@main
func main() {
    let w: Wants[&Int64] = Wants(marker: 1); // ERROR: !: Static
}
