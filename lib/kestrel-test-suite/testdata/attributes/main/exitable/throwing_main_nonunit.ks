// test: execution
// stdlib: true
// expect-exit: 7

// A throwing `@main` returning a NON-unit `Exitable` value works: `-> Int32
// throws E` desugars to `Result[Int32, E]`, and the generic
// `extend Result[T, E]: Exitable where T: Exitable` conformance produces the
// exit code from the `.Ok` value's own `report()` (here `Int32(7)` → 7).
//
// This was the v1 `Exitable` limitation (the generic `Result` conformance
// overlapped the unit-specialized one and ICE'd). It is now enabled by: making
// `()` / `!` themselves `Exitable`, generalizing the `Result` conformance, and
// E616 recursing through `Result` on its Ok type.

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
func main() -> Int32 throws MyErr {
    .Ok(7)
}
