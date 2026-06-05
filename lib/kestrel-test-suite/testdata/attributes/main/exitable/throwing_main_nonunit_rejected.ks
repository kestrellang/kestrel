// test: diagnostics
// stdlib: true
// executable: true

// v1 limitation: only the unit-Ok `Result[(), E]` conforms to `Exitable`, so a
// throwing `@main` that also returns a non-unit value (`-> Int32 throws E`,
// i.e. `Result[Int32, E]`) is rejected by E616. (Supporting it needs either a
// generic `Result[T: Exitable, E]` conformance — which overlaps the specialized
// one — or making `()` itself `Exitable`.)

module Main
import std.numeric.Int32
import std.text.Formattable
import std.text.format.StringBuilder
import std.text.format.FormatOptions

struct MyErr: Formattable {
    func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        writer.append("e");
    }
}

@main
func main() -> Int32 throws MyErr { // ERROR(E616)
    .Ok(7)
}
