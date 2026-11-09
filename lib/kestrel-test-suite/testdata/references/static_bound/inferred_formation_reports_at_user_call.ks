// test: diagnostics
// stdlib: true

// A type formed by instantiating a std signature (`collect() -> Array[Item]`
// at `Item = StringSlice`) reports its Static bound at the user's call, not
// at the signature inside iterator.ks.

module main

@main
func main() {
    let line = "a b c";
    let fields = line.split(where: { it.isWhitespace }).iter().collect(); // ERROR: StringSlice !: Static
    println(fields.count);
}
