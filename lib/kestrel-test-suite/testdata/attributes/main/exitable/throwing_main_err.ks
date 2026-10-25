// test: execution
// stdlib: true
// expect-exit: 1
// expect-stdout: before\n

// A throwing `@main` (`-> () throws E`, i.e. `Result[(), E]`) that throws: the
// error is printed to stderr and the process exits non-zero. Work before the
// throw still reaches stdout.

module Main
import std.text.(Formattable, Formatter)
import std.text.format.StringBuilder
import std.text.format.FormatOptions
import std.io.stdio.println

struct MyErr: Formattable {
    func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.append("boom");
    }
}

@main
func main() -> () throws MyErr {
     println("before");
    throw MyErr()
}
