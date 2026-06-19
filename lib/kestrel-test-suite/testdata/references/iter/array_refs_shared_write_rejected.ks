// test: diagnostics
// stdlib: true

// Stage 2d stdlib surface: `refs()` Items are SHARED — writes through
// the loop variable reject (E208), use `mutableRefs()` instead.
module Test

@main
func main() {
    let a = [1, 2, 3];
    for x in a.refs() {
        x = 5; // ERROR(E208)
    }
}
