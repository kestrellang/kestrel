// test: diagnostics
// stdlib: true
// executable: true

// `main() -> T throws E` (i.e. `Result[T, E]`) is rejected when the Ok type `T`
// does NOT conform to `Exitable`. Guards that E616 RECURSES through `Result` on
// its Ok type rather than blindly accepting any `Result` (the generic
// `Result[T, E]: Exitable where T: Exitable` conformance only applies when `T`
// is `Exitable`).
module Main
import std.text.Formattable
import std.text.format.StringBuilder
import std.text.format.FormatOptions

struct NotExitable { var x: Int64 }

struct MyErr: Formattable {
    func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        writer.append("e");
    }
}

@main
func main() -> NotExitable throws MyErr { // ERROR(E616)
    .Ok(NotExitable(x: 0))
}
