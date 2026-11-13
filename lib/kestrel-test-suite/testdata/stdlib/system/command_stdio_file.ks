// test: execution
// stdlib: true
// expect-stdout: true\nhello file\n11\ntrue\n

// `Stdio.File(path)` connects a child's stream to a file: output creates or
// truncates it, input reads it, and a missing input file is an error from
// `run()` rather than a child that sees an empty stream.

module Test

import std.files.(File, Path)

@main
func main() {
    let out = Environment.temporaryDirectory() / "kestrel-stdio-file.txt";
    var first = Command("/bin/echo");
    first.arguments(["stale", "text", "that", "gets", "truncated"]).standardOutput(.File(out.clone()));
    let _ = first.run();
    var echo = Command("/bin/echo");
    echo.arguments(["hello", "file"]).standardOutput(.File(out.clone()));
    println("\(echo.run().map { s in s.isSuccess } ?? false)");
    print(File.readText(out.clone()) ?? "<unreadable>");

    var count = Command("/usr/bin/wc");
    count.arguments(["-c"]).standardInput(.File(out.clone()));
    let counted = count.output().map { o in o.standardOutputText() ?? "" } ?? "";
    println(counted.trimmed());

    var missing = Command("/bin/cat");
    missing.standardInput(.File("/definitely/not/a/file"));
    println("\(missing.run().isErr)");

    let _ = File.remove(out);
}
