// test: execution
// stdlib: true
// expect-stdout: defaults\nnot found\n

// The `IoErrorKind` doc example: a match guard compares `e.kind` against
// an implicit member, with no `IoErrorKind` import or qualification.
module Test

import std.files.File
import std.streams.IoError

func load() -> String {
    match File.readText("/nonexistent/kestrel-test/config.toml") {
        .Ok(text) => text,
        .Err(e) if e.kind == .NotFound => "defaults",
        .Err(e) => e.description()
    }
}

@main
func main() -> Int64 {
    println(load());
    let kind = File.readText("/nonexistent/kestrel-test/x").unwrapErr().kind;
    if kind == .NotFound { println("not found") }
    0
}
