// test: execution
// stdlib: true

// Whole-file helpers, `File` with `OpenOptions`, seeking, metadata and the
// errors for a missing file, in a fresh temporary directory.

module Test

import std.streams.(IoErrorKind)

@main
func main() -> lang.i64 {
    let dir = match Directory.createTemporary(prefix: "kestrel-files") { .Ok(d) => d, .Err(_) => return 1 };
    let f = dir / Path("x.txt");

    if File.writeText(f, "hello\nworld").isErr { return 2 }
    if not f.exists() or not f.isFile() or f.isDirectory() { return 3 }
    match File.readText(f) { .Ok(t) => if t != "hello\nworld" { return 4 }, .Err(_) => return 5 }
    if File.appendText(f, "!").isErr { return 6 }
    match Metadata.of(f) {
        .Ok(m) => if m.size != 12 or m.kind != FileKind.File { return 7 },
        .Err(_) => return 8
    }

    var file = match File.open(f, OpenOptions.readingAndWriting) { .Ok(h) => h, .Err(_) => return 9 };
    match file.seek(to: .Start(6)) { .Ok(at) => if at != 6 { return 10 }, .Err(_) => return 11 }
    match file.readText() { .Ok(t) => if t != "world!" { return 12 }, .Err(_) => return 13 }
    if file.setLength(5).isErr { return 14 }
    if file.close().isErr { return 15 }
    match File.readBytes(f) { .Ok(b) => if b.count != 5 { return 16 }, .Err(_) => return 17 }

    // createNew refuses an existing file.
    var options = OpenOptions.writing;
    options.createNew = true;
    match File.open(f, options) {
        .Ok(_) => return 18,
        .Err(e) => if e.kind != IoErrorKind.AlreadyExists { return 19 }
    }

    match File.open(dir / Path("missing")) {
        .Ok(_) => return 20,
        .Err(e) => if e.kind != IoErrorKind.NotFound { return 21 }
    }
    match File.readText(Path("/definitely/not/a/real/file-kestrel-test")) {
        .Ok(_) => return 22,
        .Err(e) => if e.kind != IoErrorKind.NotFound { return 23 }
    }

    if Directory.removeAll(dir).isErr { return 24 }
    if dir.exists() { return 25 }
    0
}
