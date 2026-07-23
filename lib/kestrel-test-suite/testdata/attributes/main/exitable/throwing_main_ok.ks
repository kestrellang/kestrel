// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: ok\n

// A throwing `@main` that succeeds (`.Ok(())`) exits 0.

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
     println("ok");
    .Ok(())
}
